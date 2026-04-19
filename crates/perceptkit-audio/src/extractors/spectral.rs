//! `SpectralExtractor` — FFT-based spectral features.
//!
//! Emits:
//! - `audio.spectral_flatness`: Wiener entropy (0-1).
//!   - 1.0 = pure noise (flat spectrum)
//!   - 0.0 = pure tone
//!   - Music ≈ 0.1-0.3, Speech ≈ 0.2-0.4, White/wind ≈ 0.6-0.9
//! - `audio.spectral_centroid_hz`: center of mass of the spectrum.
//!   - Low (<500Hz) = quiet / rumble
//!   - Mid (500-2500Hz) = speech / music
//!   - High (>2500Hz) = bright / sibilant / noise
//! - `audio.spectral_rolloff_hz`: frequency below which 85% of energy lies.
//!
//! Pure Rust (`rustfft`), no external model files. Always part of default
//! `AudioProvider` after v0.2.

use std::sync::Mutex;

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use crate::extractor::FeatureExtractor;

/// Spectral features via real FFT on 50ms Hann-windowed frames.
pub struct SpectralExtractor {
    /// FFT size (default 1024 → ~64 ms @ 16 kHz).
    pub n_fft: usize,
    /// Hop size in samples (default 512).
    pub hop: usize,
    /// Rolloff percentile (default 0.85).
    pub rolloff_pct: f32,
    // Mutex for FftPlanner reuse (FFT plans are cached inside).
    fft: Mutex<std::sync::Arc<dyn Fft<f32>>>,
}

impl Default for SpectralExtractor {
    fn default() -> Self {
        Self::new(1024, 512, 0.85)
    }
}

impl SpectralExtractor {
    /// Construct with explicit FFT / hop / rolloff parameters.
    pub fn new(n_fft: usize, hop: usize, rolloff_pct: f32) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n_fft);
        Self {
            n_fft,
            hop,
            rolloff_pct,
            fft: Mutex::new(fft),
        }
    }

    /// Compute spectral flatness alone (cheaper than the full extractor).
    /// Returns Wiener entropy in [0, 1]. Useful for noise vs voice gating
    /// in the VAD without re-running the full feature pipeline.
    pub fn flatness_only(frame: &[f32], n_fft: usize) -> f32 {
        let win = Self::hann_window(n_fft);
        let mut buf: Vec<Complex32> = (0..n_fft)
            .map(|i| {
                let s = if i < frame.len() { frame[i] } else { 0.0 };
                Complex32::new(s * win[i], 0.0)
            })
            .collect();
        let mut planner = FftPlanner::<f32>::new();
        planner.plan_fft_forward(n_fft).process(&mut buf);
        let half = n_fft / 2 + 1;
        let mag: Vec<f32> = buf.iter().take(half).map(|c| c.norm()).collect();
        let slice = &mag[1..]; // skip DC bin
        if slice.is_empty() {
            return 0.0;
        }
        let n = slice.len() as f32;
        let am = slice.iter().sum::<f32>() / n;
        if am < 1e-12 {
            return 0.0;
        }
        let gm_log: f32 = slice.iter().map(|&x| (x + 1e-12).ln()).sum::<f32>() / n;
        let gm = gm_log.exp();
        (gm / am).clamp(0.0, 1.0)
    }

    fn hann_window(n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let x = (2.0 * std::f32::consts::PI * i as f32) / (n as f32 - 1.0);
                0.5 * (1.0 - x.cos())
            })
            .collect()
    }

    fn compute_frame(&self, frame: &[f32]) -> (f32, f32, f32) {
        // Zero-pad / truncate to n_fft.
        let n_fft = self.n_fft;
        let win = Self::hann_window(n_fft);
        let mut buf: Vec<Complex32> = (0..n_fft)
            .map(|i| {
                let s = if i < frame.len() { frame[i] } else { 0.0 };
                Complex32::new(s * win[i], 0.0)
            })
            .collect();

        if let Ok(fft) = self.fft.lock() {
            fft.process(&mut buf);
        }

        // Magnitude on first n_fft/2+1 bins (real signal).
        let half = n_fft / 2 + 1;
        let mag: Vec<f32> = buf.iter().take(half).map(|c| c.norm()).collect();

        let sum: f32 = mag.iter().sum();
        if sum < 1e-12 {
            return (0.0, 0.0, 0.0);
        }

        // Spectral flatness = geometric_mean / arithmetic_mean, 0..1.
        // Skip DC bin (index 0).
        let slice = &mag[1..];
        let n = slice.len() as f32;
        let am = slice.iter().sum::<f32>() / n;
        let gm_log: f32 = slice.iter().map(|&x| (x + 1e-12).ln()).sum::<f32>() / n;
        let gm = gm_log.exp();
        let flatness = if am > 1e-12 {
            (gm / am).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Spectral centroid
        let sr = 16000.0_f32;
        let bin_hz = sr / n_fft as f32;
        let num: f32 = mag
            .iter()
            .enumerate()
            .map(|(i, m)| i as f32 * bin_hz * m)
            .sum();
        let den: f32 = mag.iter().sum::<f32>();
        let centroid = if den > 1e-12 { num / den } else { 0.0 };

        // Spectral rolloff
        let threshold = sum * self.rolloff_pct;
        let mut acc = 0.0_f32;
        let mut rolloff_bin = 0;
        for (i, m) in mag.iter().enumerate() {
            acc += m;
            if acc >= threshold {
                rolloff_bin = i;
                break;
            }
        }
        let rolloff = rolloff_bin as f32 * bin_hz;

        (flatness, centroid, rolloff)
    }
}

impl FeatureExtractor for SpectralExtractor {
    fn name(&self) -> &'static str {
        "perceptkit-audio::SpectralExtractor"
    }

    fn descriptors(&self) -> Vec<FeatureDescriptor> {
        vec![
            FeatureDescriptor {
                key: FeatureKey::new("audio.spectral_flatness").unwrap(),
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
                key: FeatureKey::new("audio.spectral_centroid_hz").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(8000.0),
                },
                unit: Some("Hz".into()),
                window: TimeWindow::Sliding { ms: 1000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.spectral_rolloff_hz").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(8000.0),
                },
                unit: Some("Hz".into()),
                window: TimeWindow::Sliding { ms: 1000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION")).into(),
                version: 1,
            },
        ]
    }

    fn extract(&self, pcm: &[f32], _sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)> {
        if pcm.len() < self.n_fft {
            return vec![
                (
                    FeatureKey::new("audio.spectral_flatness").unwrap(),
                    FeatureValue::F64(0.0),
                ),
                (
                    FeatureKey::new("audio.spectral_centroid_hz").unwrap(),
                    FeatureValue::F64(0.0),
                ),
                (
                    FeatureKey::new("audio.spectral_rolloff_hz").unwrap(),
                    FeatureValue::F64(0.0),
                ),
            ];
        }

        let mut flatness_sum = 0.0_f32;
        let mut centroid_sum = 0.0_f32;
        let mut rolloff_sum = 0.0_f32;
        let mut count = 0usize;

        let mut pos = 0;
        while pos + self.n_fft <= pcm.len() {
            let frame = &pcm[pos..pos + self.n_fft];
            let (f, c, r) = self.compute_frame(frame);
            flatness_sum += f;
            centroid_sum += c;
            rolloff_sum += r;
            count += 1;
            pos += self.hop;
        }

        let n = count.max(1) as f32;
        vec![
            (
                FeatureKey::new("audio.spectral_flatness").unwrap(),
                FeatureValue::F64((flatness_sum / n) as f64),
            ),
            (
                FeatureKey::new("audio.spectral_centroid_hz").unwrap(),
                FeatureValue::F64((centroid_sum / n) as f64),
            ),
            (
                FeatureKey::new("audio.spectral_rolloff_hz").unwrap(),
                FeatureValue::F64((rolloff_sum / n) as f64),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_spectral_zero() {
        let pcm = vec![0.0_f32; 2048];
        let out = SpectralExtractor::default().extract(&pcm, 16000);
        for (_, v) in &out {
            assert_eq!(v.as_f64().unwrap(), 0.0);
        }
    }

    #[test]
    fn sine_wave_has_low_flatness() {
        // Pure tone → spectrum is peaked → flatness near 0.
        let sr = 16000.0_f32;
        let freq = 800.0_f32;
        let pcm: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin() * 0.5)
            .collect();
        let out = SpectralExtractor::default().extract(&pcm, 16000);
        let flat = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.spectral_flatness")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        assert!(flat < 0.1, "sine flatness should be low, got {flat}");
    }

    #[test]
    fn white_noise_has_high_flatness() {
        // Pseudo-random noise → flat spectrum → flatness near 1.
        let mut seed: u64 = 0xC0FFEE;
        let pcm: Vec<f32> = (0..4096)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                ((seed as u32 as f64) / (u32::MAX as f64) - 0.5) as f32 * 0.3
            })
            .collect();
        let out = SpectralExtractor::default().extract(&pcm, 16000);
        let flat = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.spectral_flatness")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        assert!(
            flat > 0.3,
            "white noise flatness should be high, got {flat}"
        );
    }

    #[test]
    fn sine_wave_centroid_near_target() {
        let sr = 16000.0_f32;
        let freq = 1000.0_f32;
        let pcm: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin() * 0.5)
            .collect();
        let out = SpectralExtractor::default().extract(&pcm, 16000);
        let centroid = out
            .iter()
            .find(|(k, _)| k.as_str() == "audio.spectral_centroid_hz")
            .unwrap()
            .1
            .as_f64()
            .unwrap();
        // 1 kHz sine; centroid should be near 1000 Hz (within bin resolution ~16 Hz).
        assert!(
            (centroid - 1000.0).abs() < 100.0,
            "centroid should be ~1000 Hz, got {centroid}"
        );
    }
}
