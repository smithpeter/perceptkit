//! `perceptkit replay` — print / inspect a `ReflectionTrace` JSONL file.
//!
//! STRATEGY §4.C: every reflection is replayable. This command renders a
//! trace file as human-readable summary.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use perceptkit_core::{read_traces, Reflection};

pub fn run(trace_file: &Path, json: bool) -> Result<ExitCode> {
    let traces =
        read_traces(trace_file).with_context(|| format!("reading {}", trace_file.display()))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&traces)?);
        return Ok(ExitCode::SUCCESS);
    }

    println!(
        "Trace: {} records from {}",
        traces.len(),
        trace_file.display()
    );
    println!();
    for (i, t) in traces.iter().enumerate() {
        let out_kind = match &t.output {
            Reflection::Map { scene_id, .. } => format!("Map → {scene_id}"),
            Reflection::Propose { .. } => "Propose (new scene)".into(),
            Reflection::Unknown { .. } => "Unknown".into(),
        };
        println!(
            "#{:03} case={} reflector={} duration={}ms → {}",
            i + 1,
            t.case_id,
            t.reflector_name,
            t.duration_ms,
            out_kind
        );
        println!("     escalation: {}", t.escalation_reason);
        if !t.input_features.is_empty() {
            println!("     input features:");
            for (k, v) in &t.input_features {
                println!("       {k} = {v}");
            }
        }
        match &t.output {
            Reflection::Map {
                scene_id,
                rationale,
            } => {
                println!("     mapped → {scene_id}");
                println!("     rationale: {rationale}");
            }
            Reflection::Propose { yaml, examples } => {
                println!("     proposed YAML:");
                for line in yaml.lines() {
                    println!("       | {line}");
                }
                println!("     examples: {}", examples.len());
            }
            Reflection::Unknown {
                summary,
                top_features,
            } => {
                println!("     summary: {summary}");
                if !top_features.is_empty() {
                    println!("     top features: {}", top_features.join(", "));
                }
            }
        }
        println!();
    }

    Ok(ExitCode::SUCCESS)
}
