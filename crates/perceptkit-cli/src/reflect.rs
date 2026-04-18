//! `perceptkit reflect` — single-invocation Reflector trigger.
//!
//! Reads a JSON feature bundle, runs `SceneEngine::evaluate_async` with the
//! configured reflector (`noop` by default), prints the SceneDecision.
//!
//! Useful for:
//! - Verifying the Cold Path wiring on a local deployment
//! - Reproducing issue reports ("when input is X, engine output should be Y")
//! - CI integration with a known bundle

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use perceptkit_core::{FeatureBundle, FeatureKey, FeatureValue, SceneEngine};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ReflectInput {
    #[serde(default)]
    features: HashMap<String, serde_json::Value>,
    #[serde(default)]
    timestamp: Option<f64>,
}

/// Run `perceptkit reflect`.
pub fn run(scenes: &std::path::Path, input: &PathBuf, json: bool) -> Result<ExitCode> {
    let engine = SceneEngine::from_dir(scenes)
        .with_context(|| format!("loading scenes from {}", scenes.display()))?;

    let content =
        std::fs::read_to_string(input).with_context(|| format!("reading {}", input.display()))?;
    let input: ReflectInput =
        serde_json::from_str(&content).with_context(|| format!("parsing {}", input.display()))?;

    let ts = input.timestamp.unwrap_or(0.0);
    let mut bundle = FeatureBundle::new(ts);
    for (k, v) in input.features {
        let key =
            FeatureKey::new(&k).map_err(|e| anyhow::anyhow!("invalid feature key '{k}': {e}"))?;
        let value = json_to_feature_value(&v).map_err(|e| anyhow::anyhow!("feature '{k}': {e}"))?;
        bundle.insert(key, value);
    }

    // Use a small tokio runtime for the async evaluate path.
    let rt = build_runtime()?;
    let decision = rt.block_on(engine.evaluate_async(&bundle));

    if json {
        let payload = serde_json::json!({
            "scene_id": decision.scene_id,
            "confidence": decision.confidence,
            "description": decision.description,
            "source": format!("{:?}", decision.source).to_lowercase(),
            "rationale": decision.rationale.iter().map(|e| e.description.clone()).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("scene_id:    {:?}", decision.scene_id);
        println!("confidence:  {:.4}", decision.confidence);
        println!("description: {:?}", decision.description);
        println!("source:      {:?}", decision.source);
        println!("rationale ({}):", decision.rationale.len());
        for ev in &decision.rationale {
            println!("  • {}", ev.description);
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn json_to_feature_value(v: &serde_json::Value) -> Result<FeatureValue, String> {
    match v {
        serde_json::Value::Number(n) => n
            .as_f64()
            .map(FeatureValue::F64)
            .ok_or_else(|| "number out of f64 range".into()),
        serde_json::Value::Bool(b) => Ok(FeatureValue::Bool(*b)),
        serde_json::Value::String(s) => Ok(FeatureValue::Category(s.clone())),
        _ => Err(format!("unsupported JSON value type: {v:?}")),
    }
}

// Minimal tokio runtime — avoid importing tokio as top-level dep.
// We shell out to the async test runtime via the `futures_executor` pattern.
fn build_runtime() -> Result<impl AsyncRt> {
    Ok(SimpleBlockOn)
}

trait AsyncRt {
    fn block_on<F: std::future::Future>(&self, f: F) -> F::Output;
}
struct SimpleBlockOn;
impl AsyncRt for SimpleBlockOn {
    fn block_on<F: std::future::Future>(&self, fut: F) -> F::Output {
        // NoopReflector never awaits anything; we can poll once.
        // For real async reflectors M6 follow-up will switch to tokio::runtime::Runtime.
        use std::pin::Pin;
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        fn noop_raw() -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| noop_raw(), |_| {}, |_| {}, |_| {});
        let waker = unsafe { Waker::from_raw(noop_raw()) };
        let mut cx = Context::from_waker(&waker);
        let mut fut = Box::pin(fut);
        loop {
            match Pin::as_mut(&mut fut).poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => {
                    // Busy-wait is fine for NoopReflector (always ready)
                    // Real async runtime is M6 follow-up.
                    std::thread::yield_now();
                }
            }
        }
    }
}
