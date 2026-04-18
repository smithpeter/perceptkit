//! Basic Rust example: load scenes, evaluate a feature bundle.
//!
//! Run from repo root:
//! ```bash
//! cargo run --example basic_rust
//! ```

use perceptkit_core::{FeatureBundle, FeatureKey, FeatureValue, SceneEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = SceneEngine::from_dir(std::path::Path::new("./scenes"))?;

    println!("Loaded {} scenes:", engine.scenes().len());
    for s in engine.scenes() {
        println!("  • {} (priority={}, v{})", s.id, s.priority, s.version);
    }

    // Build a meeting-like bundle.
    let mut bundle = FeatureBundle::new(0.0);
    bundle.insert(
        FeatureKey::new("audio.voice_ratio")?,
        FeatureValue::F64(0.72),
    );
    bundle.insert(
        FeatureKey::new("context.app")?,
        FeatureValue::Category("Zoom".into()),
    );
    bundle.insert(
        FeatureKey::new("audio.speaker_count")?,
        FeatureValue::F64(3.0),
    );
    bundle.insert(FeatureKey::new("audio.rms_db")?, FeatureValue::F64(-25.0));

    let decision = engine.evaluate(&bundle);
    println!();
    println!("Scene:       {:?}", decision.scene_id);
    println!("Confidence:  {:.2}", decision.confidence);
    println!("Description: {:?}", decision.description);
    println!("Source:      {:?}", decision.source);
    println!("Rationale ({}):", decision.rationale.len());
    for e in &decision.rationale {
        println!("  - {}", e.description);
    }

    Ok(())
}
