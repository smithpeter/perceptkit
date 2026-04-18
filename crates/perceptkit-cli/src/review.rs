//! `perceptkit review list/approve/reject` — manage PendingSceneQueue.
//!
//! STRATEGY §11.6: LLM-proposed scenes never auto-commit; the `review`
//! subcommand is the human-in-the-loop gate.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Subcommand;
use perceptkit_core::{PendingSceneQueue, PendingStatus};

/// Extract `id:` field from a minimal Scene YAML. Returns None if malformed.
fn parse_scene_id(yaml: &str) -> Option<String> {
    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("id:") {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_from_valid_yaml() {
        assert_eq!(
            parse_scene_id("id: new_scene\nversion: 1\n"),
            Some("new_scene".into())
        );
    }

    #[test]
    fn parse_id_with_quotes() {
        assert_eq!(
            parse_scene_id("id: \"quoted_scene\"\n"),
            Some("quoted_scene".into())
        );
    }

    #[test]
    fn parse_id_returns_none_if_missing() {
        assert_eq!(parse_scene_id("version: 1\ndescribe: x\n"), None);
    }
}

#[derive(Subcommand, Debug)]
pub enum ReviewCmd {
    /// List pending/approved/rejected proposals.
    List {
        /// SQLite db path.
        #[arg(long, default_value = "./pending.db")]
        db: PathBuf,
        /// Filter by status.
        #[arg(long)]
        status: Option<String>,
    },
    /// Approve a proposal — writes YAML to scenes/ and updates status.
    Approve {
        /// Proposal id.
        id: String,
        /// Reviewer user id.
        #[arg(long)]
        reviewer: String,
        /// SQLite db path.
        #[arg(long, default_value = "./pending.db")]
        db: PathBuf,
        /// Scenes directory — approved YAML is written here as `<scene_id>.yaml`.
        #[arg(long, default_value = "./scenes")]
        scenes_dir: PathBuf,
    },
    /// Reject a proposal with reason.
    Reject {
        /// Proposal id.
        id: String,
        /// Reviewer user id.
        #[arg(long)]
        reviewer: String,
        /// Reason for rejection.
        #[arg(long)]
        reason: String,
        /// SQLite db path.
        #[arg(long, default_value = "./pending.db")]
        db: PathBuf,
    },
}

pub fn run(cmd: ReviewCmd) -> Result<ExitCode> {
    match cmd {
        ReviewCmd::List { db, status } => {
            let queue = PendingSceneQueue::open(&db)
                .with_context(|| format!("opening {}", db.display()))?;
            let filter = match status.as_deref() {
                None => None,
                Some("pending") => Some(PendingStatus::Pending),
                Some("approved") => Some(PendingStatus::Approved),
                Some("rejected") => Some(PendingStatus::Rejected),
                Some(other) => anyhow::bail!("unknown status '{other}'"),
            };
            let rows = queue.list(filter)?;
            if rows.is_empty() {
                println!("no proposals");
                return Ok(ExitCode::SUCCESS);
            }
            println!("{:<24} {:<12} {:<16} reason", "id", "status", "reviewer");
            for r in rows {
                let status_str = match r.status {
                    PendingStatus::Pending => "pending",
                    PendingStatus::Approved => "approved",
                    PendingStatus::Rejected => "rejected",
                };
                println!(
                    "{:<24} {:<12} {:<16} {}",
                    r.id,
                    status_str,
                    r.reviewer.as_deref().unwrap_or("-"),
                    r.reject_reason.as_deref().unwrap_or("")
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        ReviewCmd::Approve {
            id,
            reviewer,
            db,
            scenes_dir,
        } => {
            let queue = PendingSceneQueue::open(&db)?;
            let row = queue
                .get(&id)?
                .with_context(|| format!("no proposal with id '{id}'"))?;

            // Extract scene id from YAML (parse minimal `id: x` line).
            let scene_id = parse_scene_id(&row.yaml)
                .with_context(|| "could not find `id:` field in proposed YAML")?;

            // Write YAML to scenes dir
            std::fs::create_dir_all(&scenes_dir)
                .with_context(|| format!("creating {}", scenes_dir.display()))?;
            let dest = scenes_dir.join(format!("{scene_id}.yaml"));
            if dest.exists() {
                anyhow::bail!(
                    "scenes/{scene_id}.yaml already exists — refuse to overwrite; reject or rename first"
                );
            }
            std::fs::write(&dest, &row.yaml)
                .with_context(|| format!("writing {}", dest.display()))?;

            queue.approve(&id, &reviewer)?;
            println!("✓ approved {id} → {}", dest.display());
            Ok(ExitCode::SUCCESS)
        }
        ReviewCmd::Reject {
            id,
            reviewer,
            reason,
            db,
        } => {
            let queue = PendingSceneQueue::open(&db)?;
            queue.reject(&id, &reviewer, &reason)?;
            println!("✓ rejected {id}: {reason}");
            Ok(ExitCode::SUCCESS)
        }
    }
}
