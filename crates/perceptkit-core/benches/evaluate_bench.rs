//! Criterion bench — Hot Path latency for `SceneEngine::evaluate`.
//!
//! Target (STRATEGY §2, Performance dimension): p95 < 5ms for a realistic
//! bundle of 7 features × 5 scenes.
//!
//! Run: `cargo bench -p perceptkit-core`

use std::io::Write;

use criterion::{criterion_group, criterion_main, Criterion};
use perceptkit_core::{
    FeatureBundle, FeatureKey, FeatureValue, FlappingFsm, SceneEngine, TransitionOutput,
};

fn build_scenes_dir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for (name, priority, voice_min) in [
        ("office_quiet", 5, 0.0),
        ("online_meeting", 10, 0.4),
        ("driving", 15, 0.0),
        ("outdoor_noisy", 8, 0.0),
        ("multi_speaker_chat", 7, 0.5),
    ] {
        let mut f = std::fs::File::create(tmp.path().join(format!("{name}.yaml"))).unwrap();
        writeln!(
            f,
            "id: {name}\nversion: 1\ndescribe:\n  template: {name}\nmatch:\n  all:\n    - {{ feature: audio.voice_ratio, op: gt, value: {voice_min} }}\npriority: {priority}"
        )
        .unwrap();
    }
    tmp
}

fn bench_evaluate(c: &mut Criterion) {
    let scenes_dir = build_scenes_dir();
    let engine = SceneEngine::from_dir(scenes_dir.path()).unwrap();

    let mut bundle = FeatureBundle::new(0.0);
    for (k, v) in [
        ("audio.voice_ratio", 0.72f64),
        ("audio.rms_db", -25.0),
        ("audio.peak", 0.5),
        ("audio.rms", 0.1),
        ("audio.zero_crossing_rate", 0.15),
        ("audio.speaker_count", 3.0),
    ] {
        bundle.insert(FeatureKey::new(k).unwrap(), FeatureValue::F64(v));
    }
    bundle.insert(
        FeatureKey::new("context.app").unwrap(),
        FeatureValue::Category("Zoom".into()),
    );
    bundle.insert(
        FeatureKey::new("audio.voice_activity").unwrap(),
        FeatureValue::Bool(true),
    );

    c.bench_function("engine_evaluate_8features_5scenes", |b| {
        b.iter(|| {
            let d = engine.evaluate(&bundle);
            std::hint::black_box(d);
        });
    });
}

fn bench_fsm_step(c: &mut Criterion) {
    c.bench_function("fsm_step_stable_same_scene", |b| {
        let mut fsm = FlappingFsm::default_config();
        // Warm: enter Stable("meeting")
        fsm.step(Some("meeting"), 0.80, 0.0);
        let mut t = 0.1;
        b.iter(|| {
            t += 0.1;
            let out = fsm.step(Some("meeting"), 0.75, t);
            std::hint::black_box(out);
        });
    });

    c.bench_function("fsm_step_hot_switch_transition", |b| {
        let mut fsm = FlappingFsm::default_config();
        fsm.step(Some("office"), 0.80, 0.0);
        let mut t = 0.0;
        let mut scene_idx = 0u32;
        b.iter(|| {
            t += 0.1;
            scene_idx = scene_idx.wrapping_add(1);
            let name = if scene_idx % 2 == 0 {
                "office"
            } else {
                "meeting"
            };
            let out = fsm.step(Some(name), 0.90, t);
            let is_transition = matches!(out, TransitionOutput::Transition { .. });
            std::hint::black_box((out, is_transition));
        });
    });
}

criterion_group!(benches, bench_evaluate, bench_fsm_step);
criterion_main!(benches);
