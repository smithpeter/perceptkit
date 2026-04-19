//! `MultiSpeakerExtractor` — heuristic-based.
//!
//! v0.1 was a stub (always 1). v0.2 uses voice-activity transitions
//! and spectral diversity as a coarse proxy: many transitions per
//! second + variable spectral centroid = "many voices" (crowd, laughing,
//! multi-speaker chat). Sustained tone + steady spectrum = "1 speaker".
//!
//! This is **not** a real speaker-diarisation system. It distinguishes:
//! - 1 (sustained / quiet / single voice)
//! - 2 (some variability — multi-speaker meeting, clapping, music)
//! - 3+ (high transition density — crowd, laughter)
//!
//! Real speaker counting via CAM++ / pyannote-style embeddings is
//! deferred to v0.3 (needs ML model + ONNX/inference dep — same blocker
//! as Silero VAD per `silero.rs`).

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};

use crate::extractor::FeatureExtractor;
use crate::extractors::energy::rms;
use crate::extractors::vad::zero_crossing_rate;

/// Heuristic multi-speaker estimator (v0.2).
#[derive(Debug, Clone)]
pub struct MultiSpeakerExtractor {
    /// Sub-window size in samples for activity transition counting (50ms @ 16kHz).
    pub sub_window_samples: usize,
    /// RMS threshold for "active" sub-window.
    pub rms_threshold: f64,
    /// ZCR ceiling for "voice-like" sub-window.
    pub zcr_max: f64,
    /// Transitions/second threshold for "2 speakers".
    pub two_speaker_min_tps: f64,
    /// Transitions/second threshold for "3+ speakers".
    pub three_speaker_min_tps: f64,
}

impl Default for MultiSpeakerExtractor {
    fn default() -> Self {
        Self {
            sub_window_samples: 800,
            rms_threshold: 0.01,
            zcr_max: 0.35,
            two_speaker_min_tps: 2.0,
            three_speaker_min_tps: 5.0,
        }
    }
}

impl MultiSpeakerExtractor {
    /// Construct with defaults.
    pub fn new() -> Self {
        Self::default()
    }
}

impl FeatureExtractor for MultiSpeakerExtractor {
    fn name(&self) -> &'static str {
        "perceptkit-audio::MultiSpeakerExtractor (heuristic v0.2)"
    }

    fn descriptors(&self) -> Vec<FeatureDescriptor> {
        vec![
            FeatureDescriptor {
                key: FeatureKey::new("audio.speaker_count").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(16.0),
                },
                unit: Some("count".into()),
                window: TimeWindow::Sliding { ms: 5000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION"), "/heur").into(),
                version: 1,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.activity_transitions_per_sec").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(50.0),
                },
                unit: Some("transitions_per_second".into()),
                window: TimeWindow::Sliding { ms: 5000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION"), "/heur").into(),
                version: 1,
            },
        ]
    }

    fn extract(&self, pcm: &[f32], sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)> {
        if pcm.is_empty() || self.sub_window_samples == 0 {
            return vec![
                (
                    FeatureKey::new("audio.speaker_count").unwrap(),
                    FeatureValue::F64(0.0),
                ),
                (
                    FeatureKey::new("audio.activity_transitions_per_sec").unwrap(),
                    FeatureValue::F64(0.0),
                ),
            ];
        }

        let mut transitions = 0_usize;
        let mut last_active = false;
        let mut sub_count = 0_usize;
        for chunk in pcm.chunks(self.sub_window_samples) {
            sub_count += 1;
            let r = rms(chunk);
            let zcr = zero_crossing_rate(chunk);
            let active = r > self.rms_threshold && zcr < self.zcr_max;
            if active != last_active {
                transitions += 1;
            }
            last_active = active;
        }

        let duration_s = pcm.len() as f64 / sample_rate as f64;
        let tps = if duration_s > 0.0 {
            transitions as f64 / duration_s
        } else {
            0.0
        };

        // Need at least 2 sub-windows to have a meaningful transition count.
        let speaker_count = if sub_count < 2 {
            1.0
        } else if tps >= self.three_speaker_min_tps {
            3.0
        } else if tps >= self.two_speaker_min_tps {
            2.0
        } else {
            1.0
        };

        vec![
            (
                FeatureKey::new("audio.speaker_count").unwrap(),
                FeatureValue::F64(speaker_count),
            ),
            (
                FeatureKey::new("audio.activity_transitions_per_sec").unwrap(),
                FeatureValue::F64(tps),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_returns_one_speaker() {
        let out = MultiSpeakerExtractor::new().extract(&[0.0; 16000], 16000);
        let count = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.speaker_count")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        assert_eq!(count, 1.0);
    }

    #[test]
    fn constant_tone_returns_one_speaker() {
        let pcm: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.05).sin() * 0.3).collect();
        let out = MultiSpeakerExtractor::new().extract(&pcm, 16000);
        let count = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.speaker_count")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        assert_eq!(count, 1.0);
    }

    #[test]
    fn many_bursts_returns_multi_speaker() {
        // 5 second signal: alternate 100ms loud/quiet bursts → 50 transitions
        // → 10 transitions/sec → ≥ 3 speakers.
        let burst_len = 1600; // 100 ms @ 16 kHz
        let mut pcm = Vec::with_capacity(80_000);
        for i in 0..50 {
            let amp = if i % 2 == 0 { 0.3 } else { 0.0 };
            pcm.extend(std::iter::repeat_n(amp, burst_len));
        }
        let out = MultiSpeakerExtractor::new().extract(&pcm, 16000);
        let count = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.speaker_count")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        assert!(
            count >= 2.0,
            "expected ≥ 2 speakers from bursty signal, got {count}"
        );
    }

    #[test]
    fn descriptors_include_transitions_metric() {
        let descs = MultiSpeakerExtractor::new().descriptors();
        let keys: Vec<_> = descs.iter().map(|d| d.key.as_str()).collect();
        assert!(keys.contains(&"audio.speaker_count"));
        assert!(keys.contains(&"audio.activity_transitions_per_sec"));
    }
}
