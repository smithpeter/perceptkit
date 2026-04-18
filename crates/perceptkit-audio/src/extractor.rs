//! `FeatureExtractor` trait.
//!
//! Lives in `perceptkit-audio` for v0.1. If vision / context providers emerge
//! in v0.2 and share this shape, promote to `perceptkit-core`.

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureValue};

/// Converts raw PCM audio into named `FeatureValue`s.
pub trait FeatureExtractor: Send + Sync {
    /// Identify the extractor for provenance / audit.
    fn name(&self) -> &'static str;

    /// Feature descriptors this extractor produces — register with SceneEngine.
    fn descriptors(&self) -> Vec<FeatureDescriptor>;

    /// Extract features from a PCM buffer.
    ///
    /// `pcm` — 32-bit float samples in `[-1.0, 1.0]`, mono.
    /// `sample_rate` — samples per second (typically 16000 or 48000).
    fn extract(&self, pcm: &[f32], sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)>;
}
