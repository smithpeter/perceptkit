//! `EnergyExtractor` — RMS / peak / dBFS.

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};

use crate::extractor::FeatureExtractor;

/// Produces: `audio.rms` (0-1), `audio.rms_db` (dBFS), `audio.peak` (0-1).
#[derive(Debug, Default, Clone)]
pub struct EnergyExtractor;

impl EnergyExtractor {
    /// Construct a new extractor.
    pub fn new() -> Self {
        Self
    }
}

/// RMS of a PCM buffer. Empty buffer → 0.
pub fn rms(pcm: &[f32]) -> f64 {
    if pcm.is_empty() {
        return 0.0;
    }
    let sum: f64 = pcm.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / pcm.len() as f64).sqrt()
}

/// Peak absolute amplitude.
pub fn peak(pcm: &[f32]) -> f64 {
    pcm.iter().fold(0.0_f64, |a, &s| a.max(s.abs() as f64))
}

/// Convert amplitude in `[0, 1]` → dBFS. `0.0` amplitude → `-inf` clipped to `-120`.
pub fn to_dbfs(amp: f64) -> f64 {
    if amp <= 1e-12 {
        -120.0
    } else {
        20.0 * amp.log10()
    }
}

impl FeatureExtractor for EnergyExtractor {
    fn name(&self) -> &'static str {
        "perceptkit-audio::EnergyExtractor"
    }

    fn descriptors(&self) -> Vec<FeatureDescriptor> {
        vec![
            FeatureDescriptor {
                key: FeatureKey::new("audio.rms").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                unit: Some("amplitude_0_1".into()),
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.rms_db").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(-120.0),
                    max: Some(0.0),
                },
                unit: Some("dBFS".into()),
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.peak").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                unit: Some("amplitude_0_1".into()),
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
        ]
    }

    fn extract(&self, pcm: &[f32], _sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)> {
        let rms_v = rms(pcm);
        let peak_v = peak(pcm);
        vec![
            (
                FeatureKey::new("audio.rms").unwrap(),
                FeatureValue::F64(rms_v),
            ),
            (
                FeatureKey::new("audio.rms_db").unwrap(),
                FeatureValue::F64(to_dbfs(rms_v)),
            ),
            (
                FeatureKey::new("audio.peak").unwrap(),
                FeatureValue::F64(peak_v),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_buffer_rms_zero() {
        assert_eq!(rms(&[0.0; 100]), 0.0);
        assert_eq!(peak(&[0.0; 100]), 0.0);
    }

    #[test]
    fn rms_of_constant_equals_amp() {
        let pcm = vec![0.5_f32; 1000];
        let v = rms(&pcm);
        assert!((v - 0.5).abs() < 1e-6, "rms={v}");
    }

    #[test]
    fn peak_finds_largest_abs() {
        let pcm = vec![0.1, -0.3, 0.7, -0.9, 0.2];
        assert!((peak(&pcm) - 0.9).abs() < 1e-6);
    }

    #[test]
    fn dbfs_of_half_amp_is_around_minus_6db() {
        // 20 * log10(0.5) = -6.0206
        assert!((to_dbfs(0.5) + 6.0206).abs() < 0.01);
    }

    #[test]
    fn dbfs_of_zero_is_floor() {
        assert_eq!(to_dbfs(0.0), -120.0);
    }

    #[test]
    fn extractor_emits_three_features() {
        let pcm = vec![0.5_f32; 100];
        let out = EnergyExtractor.extract(&pcm, 16000);
        assert_eq!(out.len(), 3);
        let keys: Vec<_> = out.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"audio.rms"));
        assert!(keys.contains(&"audio.rms_db"));
        assert!(keys.contains(&"audio.peak"));
    }

    #[test]
    fn descriptors_match_extracted_keys() {
        let descs = EnergyExtractor.descriptors();
        let pcm = vec![0.1_f32; 10];
        let out = EnergyExtractor.extract(&pcm, 16000);
        let desc_keys: std::collections::HashSet<_> =
            descs.iter().map(|d| d.key.as_str().to_string()).collect();
        let out_keys: std::collections::HashSet<_> =
            out.iter().map(|(k, _)| k.as_str().to_string()).collect();
        assert_eq!(desc_keys, out_keys);
    }
}
