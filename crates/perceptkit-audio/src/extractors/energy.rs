//! `EnergyExtractor` — RMS / peak / dBFS.

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};

use crate::extractor::FeatureExtractor;

/// Produces 5 features:
/// - `audio.rms` (0-1)
/// - `audio.rms_db` (absolute dBFS)
/// - `audio.peak` (0-1)
/// - `audio.rms_db_pn` (peak-normalized RMS in dB relative to peak;
///   robust across datasets with differing normalization levels)
/// - `audio.low_energy_ratio` (fraction of 50ms sub-windows whose RMS
///   is below 10% of the overall peak; high for "quiet with bursts"
///   audio like clock-ticks, near-zero for sustained sounds like engines)
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
            FeatureDescriptor {
                key: FeatureKey::new("audio.rms_db_pn").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(-120.0),
                    max: Some(0.0),
                },
                unit: Some("dB_peak_normalized".into()),
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.low_energy_ratio").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                unit: Some("ratio_0_1".into()),
                window: TimeWindow::Sliding { ms: 1000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
        ]
    }

    fn extract(&self, pcm: &[f32], _sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)> {
        let rms_v = rms(pcm);
        let peak_v = peak(pcm);
        let peak_normalized_rms = if peak_v > 1e-12 {
            to_dbfs(rms_v / peak_v)
        } else {
            -120.0
        };

        // Low-energy ratio: fraction of 50ms sub-windows whose RMS < 10% of peak.
        // 0.0 for sustained sounds; 0.5+ for transient/sparse audio.
        let sub_window = 800_usize; // 50ms @ 16 kHz
        let low_energy_ratio = if pcm.len() < sub_window || peak_v < 1e-9 {
            0.0
        } else {
            let threshold = (peak_v as f64) * 0.10;
            let mut total = 0_usize;
            let mut low = 0_usize;
            for chunk in pcm.chunks(sub_window) {
                if chunk.len() < sub_window / 2 {
                    continue;
                }
                total += 1;
                if rms(chunk) < threshold {
                    low += 1;
                }
            }
            if total == 0 {
                0.0
            } else {
                low as f64 / total as f64
            }
        };

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
            (
                FeatureKey::new("audio.rms_db_pn").unwrap(),
                FeatureValue::F64(peak_normalized_rms),
            ),
            (
                FeatureKey::new("audio.low_energy_ratio").unwrap(),
                FeatureValue::F64(low_energy_ratio),
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
    fn extractor_emits_five_features() {
        let pcm = vec![0.5_f32; 16000];
        let out = EnergyExtractor.extract(&pcm, 16000);
        assert_eq!(out.len(), 5);
        let keys: Vec<_> = out.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"audio.rms"));
        assert!(keys.contains(&"audio.rms_db"));
        assert!(keys.contains(&"audio.peak"));
        assert!(keys.contains(&"audio.rms_db_pn"));
        assert!(keys.contains(&"audio.low_energy_ratio"));
    }

    #[test]
    fn low_energy_ratio_is_zero_for_constant_signal() {
        let pcm = vec![0.5_f32; 16000];
        let v = EnergyExtractor
            .extract(&pcm, 16000)
            .into_iter()
            .find(|(k, _)| k.as_str() == "audio.low_energy_ratio")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        assert_eq!(v, 0.0, "constant signal has all sub-windows at peak");
    }

    #[test]
    fn low_energy_ratio_is_high_for_sparse_bursts() {
        // 10 sub-windows: 1 loud (peak=0.9) + 9 quiet (~0).
        let mut pcm: Vec<f32> = vec![0.0; 8000];
        for s in pcm.iter_mut().take(800) {
            *s = 0.9;
        }
        let v = EnergyExtractor
            .extract(&pcm, 16000)
            .into_iter()
            .find(|(k, _)| k.as_str() == "audio.low_energy_ratio")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        assert!(v >= 0.8, "sparse bursts → low_energy_ratio ≥ 0.8, got {v}");
    }

    #[test]
    fn rms_db_pn_is_zero_when_constant_amplitude() {
        // Constant signal: rms = peak → ratio = 1 → 0 dB peak-normalized.
        let pcm = vec![0.5_f32; 1000];
        let out = EnergyExtractor.extract(&pcm, 16000);
        let pn = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.rms_db_pn")
            .and_then(|(_, v)| v.as_f64())
            .unwrap();
        assert!(pn.abs() < 1e-3, "expected 0 dB, got {pn}");
    }

    #[test]
    fn rms_db_pn_is_lower_than_rms_db_for_dynamic_signal() {
        // Transient: peak=1.0, most samples=0 → rms << peak → rms_db_pn very negative.
        let mut pcm = vec![0.0_f32; 1000];
        pcm[500] = 1.0;
        let out = EnergyExtractor.extract(&pcm, 16000);
        let db = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.rms_db")
            .and_then(|(_, v)| v.as_f64())
            .unwrap();
        let pn = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.rms_db_pn")
            .and_then(|(_, v)| v.as_f64())
            .unwrap();
        // Peak = 1.0 → rms_db_pn == rms_db (since rms/peak == rms itself).
        assert!((db - pn).abs() < 1e-3);
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
