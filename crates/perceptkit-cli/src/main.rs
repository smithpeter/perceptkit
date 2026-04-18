//! perceptkit CLI — v0.1 commands: `lint`.
//! M6 adds: `review list/approve/reject`, `reflect`, `export`.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use perceptkit_core::SceneEngine;

#[derive(Parser, Debug)]
#[command(
    name = "perceptkit",
    version,
    about = "Perception middleware for AI agents — CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Lint a directory of scene YAML files.
    Lint {
        /// Scenes directory.
        #[arg(default_value = "./scenes")]
        path: PathBuf,
    },
    /// Show version info.
    Version,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Commands::Lint { path } => lint_cmd(&path),
        Commands::Version => {
            println!("perceptkit {}", env!("CARGO_PKG_VERSION"));
            println!("perceptkit-core {}", perceptkit_core::VERSION);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn lint_cmd(path: &std::path::Path) -> Result<ExitCode> {
    let report = SceneEngine::lint(path)
        .with_context(|| format!("linting scenes in {}", path.display()))?;

    println!("Scenes loaded: {}", report.scenes_ok);

    if !report.warnings.is_empty() {
        println!("\nWarnings:");
        for w in &report.warnings {
            println!("  ! {w}");
        }
    }

    if !report.conflicts.is_empty() {
        println!("\nConflicts:");
        for c in &report.conflicts {
            println!("  ✗ {} vs {}: {}", c.scene_a, c.scene_b, c.reason);
        }
    }

    if report.passed() {
        println!("\n✓ lint passed");
        Ok(ExitCode::SUCCESS)
    } else {
        println!(
            "\n✗ lint failed: {} conflict(s)",
            report.conflicts.len()
        );
        Ok(ExitCode::from(1))
    }
}
