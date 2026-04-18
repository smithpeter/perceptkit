//! `perceptkit review list/approve/reject` — manage PendingSceneQueue.
//!
//! STRATEGY §11.6: LLM-proposed scenes never auto-commit; the `review`
//! subcommand is the human-in-the-loop gate.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Subcommand;
use perceptkit_core::{PendingSceneQueue, PendingStatus};

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
    /// Approve a proposal.
    Approve {
        /// Proposal id.
        id: String,
        /// Reviewer user id.
        #[arg(long)]
        reviewer: String,
        /// SQLite db path.
        #[arg(long, default_value = "./pending.db")]
        db: PathBuf,
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
        ReviewCmd::Approve { id, reviewer, db } => {
            let queue = PendingSceneQueue::open(&db)?;
            queue.approve(&id, &reviewer)?;
            println!("✓ approved {id}");
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
