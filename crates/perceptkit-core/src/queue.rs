//! PendingSceneQueue — SQLite-backed queue for LLM-proposed scenes.
//!
//! Scenes proposed by Reflector go here for human review.
//! **Never** auto-committed to `scenes/*.yaml` — that's enforced at the
//! filesystem boundary via `perceptkit review approve`.

use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::{Error, Result};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS pending_scenes (
    id          TEXT PRIMARY KEY,
    created_at  REAL NOT NULL,
    case_json   TEXT NOT NULL,
    yaml        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    reviewer    TEXT,
    reviewed_at REAL,
    reject_reason TEXT
);
CREATE INDEX IF NOT EXISTS idx_pending_status ON pending_scenes(status);
";

/// Status of a pending proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingStatus {
    /// Awaiting human review.
    Pending,
    /// Approved → scene YAML written; entry kept for audit.
    Approved,
    /// Rejected → entry kept for audit + learning.
    Rejected,
}

impl PendingStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            other => Err(Error::Config(format!("unknown status '{other}'"))),
        }
    }
}

/// A row from the pending queue.
#[derive(Debug, Clone)]
pub struct PendingRow {
    /// Proposal id.
    pub id: String,
    /// Creation timestamp (unix seconds).
    pub created_at: f64,
    /// Original pending case (JSON-serialized).
    pub case_json: String,
    /// Proposed scene YAML.
    pub yaml: String,
    /// Current status.
    pub status: PendingStatus,
    /// Reviewer user id, if reviewed.
    pub reviewer: Option<String>,
    /// Review timestamp.
    pub reviewed_at: Option<f64>,
    /// Reject reason (if Rejected).
    pub reject_reason: Option<String>,
}

/// SQLite-backed pending scene queue.
pub struct PendingSceneQueue {
    conn: Connection,
}

impl PendingSceneQueue {
    /// Open (or create) the queue at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Open an in-memory queue (for tests).
    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Push a new proposal; returns assigned id.
    pub fn push(&self, id: impl Into<String>, case_json: &str, yaml: &str) -> Result<String> {
        let id = id.into();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        self.conn.execute(
            "INSERT INTO pending_scenes (id, created_at, case_json, yaml) VALUES (?1, ?2, ?3, ?4)",
            params![id, now, case_json, yaml],
        )?;
        Ok(id)
    }

    /// List rows, optionally filtered by status.
    pub fn list(&self, status: Option<PendingStatus>) -> Result<Vec<PendingRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, case_json, yaml, status, reviewer, reviewed_at, reject_reason
             FROM pending_scenes
             WHERE (?1 IS NULL OR status = ?1)
             ORDER BY created_at ASC",
        )?;
        let status_filter: Option<&str> = status.as_ref().map(|s| s.as_str());
        let rows = stmt
            .query_map(params![status_filter], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<f64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        rows.into_iter()
            .map(|(id, created_at, case_json, yaml, status, reviewer, reviewed_at, reject_reason)| {
                Ok(PendingRow {
                    id,
                    created_at,
                    case_json,
                    yaml,
                    status: PendingStatus::from_str(&status)?,
                    reviewer,
                    reviewed_at,
                    reject_reason,
                })
            })
            .collect()
    }

    /// Approve a proposal (scene YAML must still be written to disk separately).
    pub fn approve(&self, id: &str, reviewer: &str) -> Result<()> {
        self.update_status(id, PendingStatus::Approved, reviewer, None)
    }

    /// Reject a proposal with a reason.
    pub fn reject(&self, id: &str, reviewer: &str, reason: &str) -> Result<()> {
        self.update_status(id, PendingStatus::Rejected, reviewer, Some(reason))
    }

    fn update_status(
        &self,
        id: &str,
        status: PendingStatus,
        reviewer: &str,
        reject_reason: Option<&str>,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let updated = self.conn.execute(
            "UPDATE pending_scenes
             SET status = ?1, reviewer = ?2, reviewed_at = ?3, reject_reason = ?4
             WHERE id = ?5 AND status = 'pending'",
            params![status.as_str(), reviewer, now, reject_reason, id],
        )?;
        if updated == 0 {
            return Err(Error::Config(format!(
                "no pending proposal with id '{id}'"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_list() {
        let q = PendingSceneQueue::memory().unwrap();
        q.push("p1", "{}", "id: x\n").unwrap();
        let rows = q.list(None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "p1");
        assert_eq!(rows[0].status, PendingStatus::Pending);
    }

    #[test]
    fn approve_flow() {
        let q = PendingSceneQueue::memory().unwrap();
        q.push("p1", "{}", "id: x\n").unwrap();
        q.approve("p1", "alice").unwrap();
        let rows = q.list(Some(PendingStatus::Approved)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].reviewer.as_deref(), Some("alice"));
        // Double-approve fails
        assert!(q.approve("p1", "bob").is_err());
    }

    #[test]
    fn reject_with_reason() {
        let q = PendingSceneQueue::memory().unwrap();
        q.push("p1", "{}", "id: x\n").unwrap();
        q.reject("p1", "alice", "dup of scene y").unwrap();
        let rows = q.list(Some(PendingStatus::Rejected)).unwrap();
        assert_eq!(rows[0].reject_reason.as_deref(), Some("dup of scene y"));
    }

    #[test]
    fn nonexistent_id_errors() {
        let q = PendingSceneQueue::memory().unwrap();
        assert!(q.approve("nope", "x").is_err());
    }
}
