//! `SileroVadExtractor` — production-grade VAD via Silero VAD (rten backend).
//!
//! After tract and ort failures (see commit 14e795f), rten works.
//! `robertknight/rten` is a pure-Rust ML runtime with a working Silero
//! example we adapted directly. Requires `.rten`-format model:
//! ```bash
//! pip install rten-convert
//! rten-convert silero_vad.onnx silero_vad.rten
//! ```
//!
//! Signal Model preserved: rten loads from a local `.rten` file; no
//! network calls at runtime. `cargo deny` still blocks all `reqwest` /
//! `hyper` / `surf` / `ureq` / `awc` from `perceptkit-core`.
//!
//! Feature-gated by `silero-vad`. Enable with:
//! ```toml
//! perceptkit-audio = { version = "0.1", features = ["silero-vad"] }
//! ```

#![cfg(feature = "silero-vad")]

use std::path::Path;
use std::sync::Mutex;

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};
use rten::Model;
use rten_tensor::prelude::*;
use rten_tensor::NdTensor;

use crate::extractor::FeatureExtractor;

/// Silero VAD-based extractor. Loads .rten model once, runs inference
/// per chunk (default 30ms = 480 samples @ 16kHz). LSTM state threaded
/// across chunks for accurate temporal modelling.
pub struct SileroVadExtractor {
    model: Mutex<Model>,
    /// Speech probability threshold for an "active" chunk.
    pub threshold: f32,
    /// Sample rate (Silero supports 8000 and 16000; we require 16000).
    pub sample_rate: u32,
    /// Chunk size in samples (default 480 = 30ms @ 16kHz).
    pub samples_per_chunk: usize,
}

impl SileroVadExtractor {
    /// Load a Silero VAD .rten model.
    pub fn from_model_path(path: impl AsRef<Path>) -> Result<Self, rten::LoadError> {
        let model = Model::load_file(path.as_ref())?;
        Ok(Self {
            model: Mutex::new(model),
            threshold: 0.5,
            sample_rate: 16000,
            samples_per_chunk: 480, // 30ms @ 16kHz (matches rten example default)
        })
    }

    /// Override the speech-probability threshold (default 0.5).
    pub fn with_threshold(mut self, t: f32) -> Self {
        self.threshold = t;
        self
    }
}

impl FeatureExtractor for SileroVadExtractor {
    fn name(&self) -> &'static str {
        "perceptkit-audio::SileroVadExtractor (rten)"
    }

    fn descriptors(&self) -> Vec<FeatureDescriptor> {
        vec![
            FeatureDescriptor {
                key: FeatureKey::new("audio.voice_activity").unwrap(),
                kind: FeatureKind::Bool,
                unit: None,
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION"), "/silero").into(),
                version: 2,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.voice_ratio").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                unit: Some("ratio_0_1".into()),
                window: TimeWindow::Sliding { ms: 1000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION"), "/silero").into(),
                version: 2,
            },
            FeatureDescriptor {
                key: FeatureKey::new("audio.voice_prob_mean").unwrap(),
                kind: FeatureKind::F64 {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                unit: Some("probability".into()),
                window: TimeWindow::Sliding { ms: 1000 },
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION"), "/silero").into(),
                version: 1,
            },
        ]
    }

    fn extract(&self, pcm: &[f32], _sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)> {
        let keys = [
            FeatureKey::new("audio.voice_activity").unwrap(),
            FeatureKey::new("audio.voice_ratio").unwrap(),
            FeatureKey::new("audio.voice_prob_mean").unwrap(),
        ];

        if pcm.is_empty() {
            return vec![
                (keys[0].clone(), FeatureValue::Bool(false)),
                (keys[1].clone(), FeatureValue::F64(0.0)),
                (keys[2].clone(), FeatureValue::F64(0.0)),
            ];
        }

        let model = match self.model.lock() {
            Ok(m) => m,
            Err(_) => {
                tracing::warn!("silero mutex poisoned");
                return vec![
                    (keys[0].clone(), FeatureValue::Bool(false)),
                    (keys[1].clone(), FeatureValue::F64(0.0)),
                    (keys[2].clone(), FeatureValue::F64(0.0)),
                ];
            }
        };

        // Resolve node ids once per extract.
        let input_id = match model.node_id("input") {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!("silero missing 'input' node: {e}");
                return vec![
                    (keys[0].clone(), FeatureValue::Bool(false)),
                    (keys[1].clone(), FeatureValue::F64(0.0)),
                    (keys[2].clone(), FeatureValue::F64(0.0)),
                ];
            }
        };
        let sr_id = match model.node_id("sr") {
            Ok(id) => id,
            Err(_) => return vec![],
        };
        let state_id = match model.node_id("state") {
            Ok(id) => id,
            Err(_) => return vec![],
        };
        let output_id = match model.node_id("output") {
            Ok(id) => id,
            Err(_) => return vec![],
        };
        let state_n_id = match model.node_id("stateN") {
            Ok(id) => id,
            Err(_) => return vec![],
        };

        // LSTM state: [2, batch=1, 128]
        let mut state: NdTensor<f32, 3> = NdTensor::zeros([2, 1, 128]);
        let mut probs = Vec::with_capacity(pcm.len() / self.samples_per_chunk + 1);
        let sr_i32 = self.sample_rate as i32;

        for chunk in pcm.chunks(self.samples_per_chunk) {
            // Pad short trailing chunks
            let padded: Vec<f32> = chunk
                .iter()
                .copied()
                .chain(std::iter::repeat(0.0_f32))
                .take(self.samples_per_chunk)
                .collect();

            let result = model.run_n(
                [
                    (
                        input_id,
                        NdTensor::from_data([1, self.samples_per_chunk], padded).into(),
                    ),
                    (sr_id, NdTensor::from(sr_i32).into()),
                    (state_id, state.view().into()),
                ]
                .into(),
                [output_id, state_n_id],
                None,
            );

            let [output, next_state] = match result {
                Ok(arr) => arr,
                Err(e) => {
                    tracing::warn!("silero run failed: {e}");
                    continue;
                }
            };

            let prob_t: NdTensor<f32, 2> = match output.try_into() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let prob = prob_t.get([0, 0]).copied().unwrap_or(0.0);
            probs.push(prob);

            state = match next_state.try_into() {
                Ok(s) => s,
                Err(_) => continue,
            };
        }

        if probs.is_empty() {
            return vec![
                (keys[0].clone(), FeatureValue::Bool(false)),
                (keys[1].clone(), FeatureValue::F64(0.0)),
                (keys[2].clone(), FeatureValue::F64(0.0)),
            ];
        }

        let active_frames = probs.iter().filter(|&&p| p > self.threshold).count();
        let voice_ratio = active_frames as f64 / probs.len() as f64;
        let voice_active = voice_ratio > 0.0;
        let mean_prob = probs.iter().sum::<f32>() as f64 / probs.len() as f64;

        vec![
            (keys[0].clone(), FeatureValue::Bool(voice_active)),
            (keys[1].clone(), FeatureValue::F64(voice_ratio)),
            (keys[2].clone(), FeatureValue::F64(mean_prob)),
        ]
    }
}
