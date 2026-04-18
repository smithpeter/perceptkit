//! `perceptkit eval` — run the engine on a labeled JSONL dataset and report
//! macro-F1 / Top-1 / per-scene recall.
//!
//! Dataset format (one JSON object per line):
//! ```json
//! {"features": {"audio.voice_ratio": 0.72, "context.app": "Zoom"}, "label": "online_meeting"}
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use perceptkit_core::{FeatureBundle, FeatureKey, FeatureValue, SceneEngine};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct EvalRow {
    features: HashMap<String, serde_json::Value>,
    label: String,
}

/// Per-class classification metrics.
#[derive(Debug, Clone, Default)]
pub struct ClassMetrics {
    pub tp: usize,
    pub fp: usize,
    pub fn_: usize,
    pub support: usize,
}

impl ClassMetrics {
    fn precision(&self) -> f64 {
        if self.tp + self.fp == 0 {
            0.0
        } else {
            self.tp as f64 / (self.tp + self.fp) as f64
        }
    }
    fn recall(&self) -> f64 {
        if self.tp + self.fn_ == 0 {
            0.0
        } else {
            self.tp as f64 / (self.tp + self.fn_) as f64
        }
    }
    fn f1(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }
}

/// Aggregate evaluation result.
#[derive(Debug)]
pub struct EvalReport {
    pub n: usize,
    pub top1_accuracy: f64,
    pub macro_f1: f64,
    pub per_class: HashMap<String, ClassMetrics>,
    pub unknown_count: usize,
}

impl EvalReport {
    /// Whether the report meets v0.1 gates (plan.md §4.5).
    pub fn passes_v01_gate(&self) -> bool {
        const TOP1_MIN: f64 = 0.78;
        const MACRO_F1_MIN: f64 = 0.72;
        const PER_SCENE_MIN: f64 = 0.70;
        self.top1_accuracy >= TOP1_MIN
            && self.macro_f1 >= MACRO_F1_MIN
            && self
                .per_class
                .values()
                .all(|m| m.support == 0 || m.recall() >= PER_SCENE_MIN)
    }
}

/// Run evaluation from CLI.
pub fn eval_cmd(scenes: &Path, dataset: &Path, gate: bool) -> Result<ExitCode> {
    let engine = SceneEngine::from_dir(scenes)
        .with_context(|| format!("loading scenes from {}", scenes.display()))?;
    let report = evaluate(&engine, dataset)?;

    println!("Evaluated {} samples", report.n);
    println!("Top-1 accuracy: {:.4}", report.top1_accuracy);
    println!("Macro-F1:       {:.4}", report.macro_f1);
    println!("Unknown predictions: {}", report.unknown_count);
    println!();
    println!("Per-scene:");
    println!(
        "  {:<24} {:>8} {:>8} {:>8} {:>8}",
        "scene", "support", "prec", "recall", "f1"
    );
    let mut names: Vec<_> = report.per_class.keys().collect();
    names.sort();
    for name in names {
        let m = &report.per_class[name];
        println!(
            "  {:<24} {:>8} {:>8.4} {:>8.4} {:>8.4}",
            name,
            m.support,
            m.precision(),
            m.recall(),
            m.f1()
        );
    }

    if gate {
        if report.passes_v01_gate() {
            println!("\n✓ v0.1 gate passed (Top-1 ≥0.78, Macro-F1 ≥0.72, per-scene recall ≥0.70)");
            Ok(ExitCode::SUCCESS)
        } else {
            println!("\n✗ v0.1 gate failed");
            Ok(ExitCode::from(1))
        }
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Programmatic evaluation (reusable from tests).
pub fn evaluate(engine: &SceneEngine, dataset: &Path) -> Result<EvalReport> {
    let file = File::open(dataset).with_context(|| format!("opening {}", dataset.display()))?;
    let reader = BufReader::new(file);

    let mut per_class: HashMap<String, ClassMetrics> = HashMap::new();
    let mut correct = 0_usize;
    let mut total = 0_usize;
    let mut unknown_count = 0_usize;

    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let row: EvalRow = serde_json::from_str(trimmed)
            .with_context(|| format!("parsing line {} in {}", line_no + 1, dataset.display()))?;

        let mut bundle = FeatureBundle::new(0.0);
        for (k, v) in row.features {
            let key = FeatureKey::new(&k)
                .with_context(|| format!("invalid feature key '{k}' at line {}", line_no + 1))?;
            let value = json_to_feature_value(&v)
                .with_context(|| format!("unsupported value for '{k}' at line {}", line_no + 1))?;
            bundle.insert(key, value);
        }

        let decision = engine.evaluate(&bundle);
        let truth = &row.label;
        let pred = decision.scene_id.as_deref();

        per_class.entry(truth.clone()).or_default().support += 1;

        total += 1;
        match pred {
            None => unknown_count += 1,
            Some(p) => {
                if p == truth {
                    correct += 1;
                    per_class.entry(truth.clone()).or_default().tp += 1;
                } else {
                    per_class.entry(p.to_string()).or_default().fp += 1;
                    per_class.entry(truth.clone()).or_default().fn_ += 1;
                }
            }
        }
    }

    let top1 = if total == 0 {
        0.0
    } else {
        correct as f64 / total as f64
    };

    // Macro-F1 over classes with non-zero support (real ground-truth labels).
    let labeled: Vec<&ClassMetrics> = per_class.values().filter(|m| m.support > 0).collect();
    let macro_f1 = if labeled.is_empty() {
        0.0
    } else {
        labeled.iter().map(|m| m.f1()).sum::<f64>() / labeled.len() as f64
    };

    Ok(EvalReport {
        n: total,
        top1_accuracy: top1,
        macro_f1,
        per_class,
        unknown_count,
    })
}

fn json_to_feature_value(v: &serde_json::Value) -> Result<FeatureValue> {
    match v {
        serde_json::Value::Number(n) => {
            let f = n.as_f64().context("number out of f64 range")?;
            Ok(FeatureValue::F64(f))
        }
        serde_json::Value::Bool(b) => Ok(FeatureValue::Bool(*b)),
        serde_json::Value::String(s) => Ok(FeatureValue::Category(s.clone())),
        _ => anyhow::bail!("unsupported JSON value type: {v:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_metrics_f1() {
        let m = ClassMetrics {
            tp: 8,
            fp: 2,
            fn_: 2,
            support: 10,
        };
        assert!((m.precision() - 0.8).abs() < 1e-6);
        assert!((m.recall() - 0.8).abs() < 1e-6);
        assert!((m.f1() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn class_metrics_zero_division_safe() {
        let m = ClassMetrics::default();
        assert_eq!(m.precision(), 0.0);
        assert_eq!(m.recall(), 0.0);
        assert_eq!(m.f1(), 0.0);
    }
}
