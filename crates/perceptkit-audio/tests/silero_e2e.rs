//! E2E test: load Silero VAD .rten model and verify it distinguishes speech from noise.
//!
//! Only runs when `PERCEPTKIT_SILERO_MODEL` env points to a valid .rten file.
//!
//! Run:
//! ```bash
//! PERCEPTKIT_SILERO_MODEL=./models/silero_vad.rten \
//!     cargo test -p perceptkit-audio --features silero-vad \
//!     --test silero_e2e -- --nocapture
//! ```

#![cfg(feature = "silero-vad")]

use perceptkit_audio::{FeatureExtractor, SileroVadExtractor};
use perceptkit_core::FeatureValue;

fn model_path() -> Option<String> {
    std::env::var("PERCEPTKIT_SILERO_MODEL")
        .ok()
        .filter(|p| std::path::Path::new(p).exists())
}

fn sine_wave(hz: f64, sample_rate: u32, samples: usize, amp: f32) -> Vec<f32> {
    (0..samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            (t * hz * 2.0 * std::f64::consts::PI).sin() as f32 * amp
        })
        .collect()
}

fn get_voice_ratio(out: &[(perceptkit_core::FeatureKey, FeatureValue)]) -> f64 {
    out.iter()
        .find(|(k, _)| k.as_str() == "audio.voice_ratio")
        .and_then(|(_, v)| v.as_f64())
        .unwrap_or(-1.0)
}

#[test]
fn silero_silence_has_low_voice_ratio() {
    let Some(path) = model_path() else {
        eprintln!("skipped: set PERCEPTKIT_SILERO_MODEL to run");
        return;
    };
    let ex = SileroVadExtractor::from_model_path(&path).expect("load model");
    let pcm = vec![0.0_f32; 16000];
    let out = ex.extract(&pcm, 16000);
    let voice_ratio = get_voice_ratio(&out);
    println!("silence → voice_ratio = {voice_ratio}");
    assert!(
        voice_ratio < 0.1,
        "silence should have near-zero voice, got {voice_ratio}"
    );
}

#[test]
fn silero_white_noise_has_low_voice_ratio() {
    let Some(path) = model_path() else {
        return;
    };
    let ex = SileroVadExtractor::from_model_path(&path).expect("load model");
    let mut seed: u64 = 0xDEAD_BEEF_CAFE_F00D;
    let pcm: Vec<f32> = (0..16000)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            ((seed as u32 as f64) / (u32::MAX as f64) - 0.5) as f32 * 0.3
        })
        .collect();
    let out = ex.extract(&pcm, 16000);
    let voice_ratio = get_voice_ratio(&out);
    println!("white noise → voice_ratio = {voice_ratio}");
    assert!(
        voice_ratio < 0.4,
        "white noise should have low voice_ratio, got {voice_ratio}"
    );
}

#[test]
fn silero_sine_wave_runs_without_panic() {
    let Some(path) = model_path() else {
        return;
    };
    let ex = SileroVadExtractor::from_model_path(&path).expect("load model");
    let pcm = sine_wave(200.0, 16000, 16000, 0.3);
    let out = ex.extract(&pcm, 16000);
    let voice_ratio = get_voice_ratio(&out);
    println!("200Hz sine → voice_ratio = {voice_ratio}");
    assert!((0.0..=1.0).contains(&voice_ratio));
}

#[test]
fn silero_emits_three_features() {
    let Some(path) = model_path() else {
        return;
    };
    let ex = SileroVadExtractor::from_model_path(&path).expect("load model");
    let pcm = vec![0.0_f32; 512];
    let out = ex.extract(&pcm, 16000);
    let keys: Vec<_> = out.iter().map(|(k, _)| k.as_str()).collect();
    assert!(keys.contains(&"audio.voice_activity"));
    assert!(keys.contains(&"audio.voice_ratio"));
    assert!(keys.contains(&"audio.voice_prob_mean"));
}
