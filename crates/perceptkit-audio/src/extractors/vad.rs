//! `VoiceActivityExtractor` — v0.1 minimal VAD based on energy + zero-crossing rate.
//!
//! Intentionally simple. v0.2 will replace with Silero VAD (ONNX) behind a feature flag.

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};

use crate::extractor::FeatureExtractor;
use crate::extractors::energy::rms;

/// Voice activity detector (energy + ZCR threshold).
///
/// Produces:
/// - `audio.voice_activity` (bool)
/// - `audio.voice_ratio` (0-1, fraction of sub-windows detected as voice)
/// - `audio.zero_crossing_rate` (0-1, per sample)
#[derive(Debug, Clone)]
pub struct VoiceActivityExtractor {
    /// RMS threshold above which a frame is considered active.
    pub rms_threshold: f64,
    /// Zero-crossing rate upper bound for speech (higher = noise-like).
    pub zcr_max: f64,
    /// Sub-window size in samples for voice_ratio computation.
    pub sub_window_samples: usize,
}

impl Default for VoiceActivityExtractor {
    fn default() -> Self {
        Self {
            rms_threshold: 0.01,
            zcr_max: 0.35,
            sub_window_samples: 800, // 50 ms @ 16 kHz
        }
    }
}

impl VoiceActivityExtractor {
    /// Construct with default thresholds.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Zero-crossing rate per sample.
pub fn zero_crossing_rate(pcm: &[f32]) -> f64 {
    if pcm.len() < 2 {
        return 0.0;
    }
    let mut crossings = 0_usize;
    for w in pcm.windows(2) {
        if (w[0] >= 0.0) != (w[1] >= 0.0) {
            crossings += 1;
        }
    }
    crossings as f64 / (pcm.len() - 1) as f64
}

impl FeatureExtractor for VoiceActivityExtractor {
    fn name(&self) -> &'static str {
        "perceptkit-audio::VoiceActivityExtractor"
    }

    fn descriptors(&self) -> Vec<FeatureDescriptor> {
        vec![
            FeatureDescriptor {
                key: FeatureKey::new("audio.voice_activity").unwrap(),
                kind: FeatureKind::Bool,
                unit: None,
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.voice_ratio").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                unit: Some("ratio_0_1".into()),
                window: TimeWindow::Sliding { ms: 1000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.zero_crossing_rate").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                unit: Some("ratio_0_1".into()),
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
        ]
    }

    fn extract(&self, pcm: &[f32], _sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)> {
        let rms_v = rms(pcm);
        let zcr_v = zero_crossing_rate(pcm);
        let active_overall = rms_v > self.rms_threshold && zcr_v < self.zcr_max;

        // Compute voice_ratio over sub-windows.
        let voice_ratio = if pcm.is_empty() || self.sub_window_samples == 0 {
            0.0
        } else {
            let mut active = 0_usize;
            let mut total = 0_usize;
            for chunk in pcm.chunks(self.sub_window_samples) {
                total += 1;
                let sub_rms = rms(chunk);
                let sub_zcr = zero_crossing_rate(chunk);
                if sub_rms > self.rms_threshold && sub_zcr < self.zcr_max {
                    active += 1;
                }
            }
            if total == 0 {
                0.0
            } else {
                active as f64 / total as f64
            }
        };

        vec![
            (
                FeatureKey::new("audio.voice_activity").unwrap(),
                FeatureValue::Bool(active_overall),
            ),
            (
                FeatureKey::new("audio.voice_ratio").unwrap(),
                FeatureValue::F64(voice_ratio),
            ),
            (
                FeatureKey::new("audio.zero_crossing_rate").unwrap(),
                FeatureValue::F64(zcr_v),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(hz: f64, sample_rate: u32, samples: usize, amp: f32) -> Vec<f32> {
        (0..samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                (t * hz * 2.0 * std::f64::consts::PI).sin() as f32 * amp
            })
            .collect()
    }

    #[test]
    fn silence_reports_no_activity() {
        let pcm = vec![0.0_f32; 16000];
        let out = VoiceActivityExtractor::new().extract(&pcm, 16000);
        let m: std::collections::HashMap<_, _> = out
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v))
            .collect();
        assert_eq!(m["audio.voice_activity"].as_bool(), Some(false));
        assert_eq!(m["audio.voice_ratio"].as_f64(), Some(0.0));
    }

    #[test]
    fn sine_wave_at_speech_freq_detected_as_voice() {
        // 200 Hz sine at amplitude 0.3 — low ZCR, reasonable energy.
        let pcm = sine_wave(200.0, 16000, 16000, 0.3);
        let out = VoiceActivityExtractor::new().extract(&pcm, 16000);
        let m: std::collections::HashMap<_, _> = out
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v))
            .collect();
        assert_eq!(m["audio.voice_activity"].as_bool(), Some(true));
        assert!(m["audio.voice_ratio"].as_f64().unwrap() > 0.9);
    }

    #[test]
    fn white_noise_high_zcr_rejected() {
        // High-ZCR noise (alternating signs) — rejected as non-voice.
        let pcm: Vec<f32> = (0..16000)
            .map(|i| if i % 2 == 0 { 0.3 } else { -0.3 })
            .collect();
        let out = VoiceActivityExtractor::new().extract(&pcm, 16000);
        let m: std::collections::HashMap<_, _> = out
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v))
            .collect();
        assert_eq!(m["audio.voice_activity"].as_bool(), Some(false));
    }

    #[test]
    fn zcr_of_alternating_signs_is_one() {
        let pcm: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let v = zero_crossing_rate(&pcm);
        assert!((v - 1.0).abs() < 1e-6);
    }
}
