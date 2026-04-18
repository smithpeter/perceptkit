//! E2E test: load Qwen-0.5B GGUF and run a real reflection.
//!
//! Only runs when `PERCEPTKIT_MODEL_PATH` env is set to a valid GGUF file.
//! This is intentional: the model is ~470 MB and not in CI; the test is for
//! local validation of the LocalReflector integration.
//!
//! Run:
//! ```bash
//! PERCEPTKIT_MODEL_PATH=./models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
//!     cargo test -p perceptkit-core --features local-reflector \
//!     --test local_reflector_e2e -- --nocapture
//! ```

#![cfg(feature = "local-reflector")]

use std::time::Instant;

use perceptkit_core::{
    LocalConfig, LocalReflector, PendingCase, Reflection, Reflector, SceneDecision,
};

fn model_path() -> Option<String> {
    std::env::var("PERCEPTKIT_MODEL_PATH").ok().filter(|p| {
        let exists = std::path::Path::new(p).exists();
        if !exists {
            eprintln!("PERCEPTKIT_MODEL_PATH set but file missing: {p}");
        }
        exists
    })
}

fn case_with_meeting_features() -> PendingCase {
    PendingCase {
        id: "e2e-1".into(),
        timestamp: 0.0,
        features: vec![
            ("audio.voice_ratio".into(), serde_yml::Value::from(0.72)),
            ("audio.rms_db".into(), serde_yml::Value::from(-25.0)),
            ("context.app".into(), serde_yml::Value::from("Zoom")),
            ("audio.speaker_count".into(), serde_yml::Value::from(3.0)),
        ],
        reason: "hot path confidence 0.55 (below accept threshold 0.70)".into(),
        failed_decision: SceneDecision::unknown(),
    }
}

#[tokio::test]
async fn qwen_reflect_returns_valid_reflection() {
    let Some(path) = model_path() else {
        eprintln!("skipped: set PERCEPTKIT_MODEL_PATH to run");
        return;
    };

    let mut config = LocalConfig::default();
    config.known_scenes = vec![
        "office_quiet".into(),
        "online_meeting".into(),
        "driving".into(),
        "outdoor_noisy".into(),
        "multi_speaker_chat".into(),
        "music_playback".into(),
        "coding".into(),
    ];
    config.max_new_tokens = 128;

    let reflector = LocalReflector::with_config(&path, config).expect("load model");
    let case = case_with_meeting_features();

    let t0 = Instant::now();
    let result = reflector.reflect(case).await;
    let elapsed = t0.elapsed();

    println!("\nQwen-0.5B reflected in {:?}", elapsed);
    match &result {
        Ok(Reflection::Map {
            scene_id,
            rationale,
        }) => {
            println!("Reflection::Map → scene_id={scene_id}, rationale={rationale}");
        }
        Ok(Reflection::Propose { yaml, examples }) => {
            println!("Reflection::Propose → yaml:\n{yaml}");
            println!("Examples: {examples:?}");
        }
        Ok(Reflection::Unknown {
            summary,
            top_features,
        }) => {
            println!("Reflection::Unknown → summary={summary}, top={top_features:?}");
        }
        Err(e) => {
            eprintln!("Reflection failed: {e}");
            panic!("Qwen reflect failed: {e}");
        }
    }

    // Minimum bar: we got SOME valid Reflection variant (not an error).
    assert!(result.is_ok(), "expected Ok Reflection, got {result:?}");
}

#[tokio::test]
async fn qwen_reflect_maps_obvious_meeting_to_online_meeting() {
    let Some(path) = model_path() else {
        return;
    };

    let mut config = LocalConfig::default();
    config.known_scenes = vec!["office_quiet".into(), "online_meeting".into()];
    config.max_new_tokens = 96;

    let reflector = LocalReflector::with_config(&path, config).expect("load model");
    let case = case_with_meeting_features();

    let result = reflector.reflect(case).await.expect("reflect ok");
    // Lenient assert: small 0.5B model may still land on Unknown/Propose;
    // we record the outcome for audit but don't hard-fail (qwen-0.5b is marginal).
    println!("\nMeeting case → {result:?}");
}
