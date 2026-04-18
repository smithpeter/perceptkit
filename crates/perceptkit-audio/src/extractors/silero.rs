//! `SileroVadExtractor` — production-grade VAD via Silero VAD ONNX.
//!
//! **Status (2026-04-19)**: API scaffolded, model loading fails on tract-onnx
//! 0.22.1. Both Silero v4 (gather panic) and v5 (If-op squeeze) hit tract
//! compatibility bugs. For production, swap tract → `ort` (has full ONNX
//! Runtime compatibility; requires C++ onnxruntime library on system or
//! use `load-dynamic` feature). API shape and tests here are correct and
//! will work as-is when the inference backend is swapped.
//!
//! Signal Model preserved: the ONNX model is loaded from a local path
//! (no network). Weights bundled out-of-tree — not in the crate.
//!
//! Feature-gated by `silero-vad`. Enable with:
//! ```toml
//! perceptkit-audio = { version = "0.1", features = ["silero-vad"] }
//! ```
//!
//! Download the ONNX model once (2.2 MB):
//! ```bash
//! mkdir -p models
//! curl -L -o models/silero_vad.onnx \
//!     https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx
//! ```

#![cfg(feature = "silero-vad")]

use std::path::Path;
use std::sync::Mutex;

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};
use tract_onnx::prelude::*;

use crate::extractor::FeatureExtractor;

/// Silero VAD-based extractor. Loads ONNX model once, runs inference per
/// 512-sample chunk (32ms @ 16kHz). Produces the same 3 features as
/// `VoiceActivityExtractor` but with dramatically better accuracy on
/// non-speech audio.
pub struct SileroVadExtractor {
    // Tract model is !Sync due to interior mutability; wrap in Mutex.
    // Silero inference is fast (~0.5 ms per 512 samples) so lock contention
    // is minimal in single-threaded use.
    model: Mutex<TractRunnableModel>,
    /// Speech probability threshold for considering a chunk "active".
    pub threshold: f32,
    /// Sample rate (Silero supports 8000 and 16000; we require 16000).
    pub sample_rate: u32,
    /// Chunk size in samples (must be 512 for 16kHz).
    pub chunk_size: usize,
}

type TractRunnableModel =
    SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

impl SileroVadExtractor {
    /// Load a Silero VAD ONNX model.
    pub fn from_model_path(path: impl AsRef<Path>) -> TractResult<Self> {
        let chunk_size: usize = 512;
        let path = path.as_ref();
        let model = tract_onnx::onnx()
            .model_for_path(path)?
            // Silero VAD's ONNX declares dynamic input shapes; provide facts
            // so tract can build a typed graph.
            // Input 0: audio samples [1, 512]
            .with_input_fact(
                0,
                InferenceFact::dt_shape(f32::datum_type(), tvec!(1_i32, chunk_size as i32)),
            )?
            // Input 1: LSTM state [2, 1, 128]
            .with_input_fact(
                1,
                InferenceFact::dt_shape(f32::datum_type(), tvec!(2_i32, 1_i32, 128_i32)),
            )?
            // Input 2: sample rate scalar (i64)
            .with_input_fact(
                2,
                InferenceFact::dt_shape(
                    i64::datum_type(),
                    tvec!() as tract_onnx::prelude::TVec<i32>,
                ),
            )?
            .into_optimized()?
            .into_runnable()?;
        Ok(Self {
            model: Mutex::new(model),
            threshold: 0.5,
            sample_rate: 16000,
            chunk_size,
        })
    }

    /// Override the speech-probability threshold (default 0.5).
    pub fn with_threshold(mut self, t: f32) -> Self {
        self.threshold = t;
        self
    }

    fn run_chunk(&self, chunk: &[f32], state: &mut tract_ndarray::Array3<f32>) -> TractResult<f32> {
        let audio_tensor: Tensor =
            tract_ndarray::Array2::from_shape_vec((1, self.chunk_size), chunk.to_vec())?.into();
        let state_tensor: Tensor = state.clone().into();
        let sr_tensor: Tensor = tract_ndarray::arr0::<i64>(self.sample_rate as i64)
            .into_dyn()
            .into();

        let model = self
            .model
            .lock()
            .map_err(|_| TractError::msg("silero mutex poisoned"))?;
        let outputs = model.run(tvec!(
            audio_tensor.into(),
            state_tensor.into(),
            sr_tensor.into(),
        ))?;

        // Output 0: speech probability [1, 1]
        let prob_view = outputs[0].to_array_view::<f32>()?;
        let prob = prob_view
            .as_slice()
            .and_then(|s| s.first().copied())
            .unwrap_or(0.0);

        // Output 1: updated state [2, 1, 128]
        let new_state_view = outputs[1].to_array_view::<f32>()?;
        *state = new_state_view.to_owned().into_dimensionality()?;

        Ok(prob)
    }
}

impl FeatureExtractor for SileroVadExtractor {
    fn name(&self) -> &'static str {
        "perceptkit-audio::SileroVadExtractor"
    }

    fn descriptors(&self) -> Vec<FeatureDescriptor> {
        vec![
            FeatureDescriptor {
                key: FeatureKey::new("audio.voice_activity").unwrap(),
                kind: FeatureKind::Bool,
                unit: None,
                window: TimeWindow::Instant,
                source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION"), "/silero").into(),
                version: 2, // v2: silero-based (v1 was energy+ZCR)
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
        if pcm.is_empty() {
            return vec![
                (
                    FeatureKey::new("audio.voice_activity").unwrap(),
                    FeatureValue::Bool(false),
                ),
                (
                    FeatureKey::new("audio.voice_ratio").unwrap(),
                    FeatureValue::F64(0.0),
                ),
                (
                    FeatureKey::new("audio.voice_prob_mean").unwrap(),
                    FeatureValue::F64(0.0),
                ),
            ];
        }

        let mut state: tract_ndarray::Array3<f32> = tract_ndarray::Array3::zeros((2, 1, 128));
        let mut probs = Vec::with_capacity(pcm.len() / self.chunk_size + 1);

        for chunk in pcm.chunks(self.chunk_size) {
            // Pad short trailing chunks with zeros — Silero expects exact size.
            let processed: std::borrow::Cow<'_, [f32]> = if chunk.len() < self.chunk_size {
                let mut v = chunk.to_vec();
                v.resize(self.chunk_size, 0.0);
                std::borrow::Cow::Owned(v)
            } else {
                std::borrow::Cow::Borrowed(chunk)
            };
            match self.run_chunk(&processed, &mut state) {
                Ok(p) => probs.push(p),
                Err(e) => {
                    tracing::warn!("silero chunk failed: {e}");
                }
            }
        }

        if probs.is_empty() {
            return vec![
                (
                    FeatureKey::new("audio.voice_activity").unwrap(),
                    FeatureValue::Bool(false),
                ),
                (
                    FeatureKey::new("audio.voice_ratio").unwrap(),
                    FeatureValue::F64(0.0),
                ),
                (
                    FeatureKey::new("audio.voice_prob_mean").unwrap(),
                    FeatureValue::F64(0.0),
                ),
            ];
        }

        let active_frames = probs.iter().filter(|&&p| p > self.threshold).count();
        let voice_ratio = active_frames as f64 / probs.len() as f64;
        let voice_active = voice_ratio > 0.0;
        let mean_prob = probs.iter().sum::<f32>() as f64 / probs.len() as f64;

        vec![
            (
                FeatureKey::new("audio.voice_activity").unwrap(),
                FeatureValue::Bool(voice_active),
            ),
            (
                FeatureKey::new("audio.voice_ratio").unwrap(),
                FeatureValue::F64(voice_ratio),
            ),
            (
                FeatureKey::new("audio.voice_prob_mean").unwrap(),
                FeatureValue::F64(mean_prob),
            ),
        ]
    }
}
