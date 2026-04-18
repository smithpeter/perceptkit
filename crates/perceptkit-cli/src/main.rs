//! perceptkit CLI — v0.1 commands: `lint`, `eval`, `synthesize`, `version`.
//! M6 adds: `review list/approve/reject`, `reflect`, `export`.

mod eval;
mod reflect;
mod replay;
mod review;
mod synth;

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
    /// Evaluate engine on a labeled JSONL dataset (macro-F1 / Top-1 / per-scene).
    Eval {
        /// Scenes directory.
        #[arg(long, default_value = "./scenes")]
        scenes: PathBuf,
        /// JSONL dataset path ({features, label} per line).
        #[arg(long)]
        dataset: PathBuf,
        /// Exit non-zero if v0.1 gates (Top-1 ≥0.78, Macro-F1 ≥0.72) fail.
        #[arg(long, default_value_t = false)]
        gate: bool,
    },
    /// Synthesize a labeled JSONL dataset for CI smoke tests.
    Synthesize {
        /// Output JSONL path.
        #[arg(long)]
        out: PathBuf,
        /// Rows per scene (total = 5 × per_scene).
        #[arg(long, default_value_t = 50)]
        per_scene: usize,
        /// PRNG seed for deterministic output.
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Manage pending LLM-proposed scenes (human-in-the-loop review).
    Review {
        #[command(subcommand)]
        cmd: review::ReviewCmd,
    },
    /// Single-invocation Reflector trigger — load scenes, eval async, print decision.
    Reflect {
        /// Scenes directory.
        #[arg(long, default_value = "./scenes")]
        scenes: PathBuf,
        /// Input JSON file ({"features": {...}, "timestamp": 0.0}).
        #[arg(long)]
        input: PathBuf,
        /// Output as JSON (default: human-readable).
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Render a ReflectionTrace JSONL file for audit / debugging.
    Replay {
        /// Trace file path (JSONL).
        #[arg(long)]
        trace: PathBuf,
        /// Output as JSON (default: human-readable summary).
        #[arg(long, default_value_t = false)]
        json: bool,
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
        Commands::Eval {
            scenes,
            dataset,
            gate,
        } => eval::eval_cmd(&scenes, &dataset, gate),
        Commands::Synthesize {
            out,
            per_scene,
            seed,
        } => synth::synthesize_cmd(&out, per_scene, seed),
        Commands::Review { cmd } => review::run(cmd),
        Commands::Reflect {
            scenes,
            input,
            json,
        } => reflect::run(&scenes, &input, json),
        Commands::Replay { trace, json } => replay::run(&trace, json),
        Commands::Version => {
            println!("perceptkit {}", env!("CARGO_PKG_VERSION"));
            println!("perceptkit-core {}", perceptkit_core::VERSION);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn lint_cmd(path: &std::path::Path) -> Result<ExitCode> {
    let report =
        SceneEngine::lint(path).with_context(|| format!("linting scenes in {}", path.display()))?;

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
        println!("\n✗ lint failed: {} conflict(s)", report.conflicts.len());
        Ok(ExitCode::from(1))
    }
}
