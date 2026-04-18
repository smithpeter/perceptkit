//! `MultiSpeakerExtractor` — v0.1 stub.
//!
//! v0.2 will integrate CAM++ / pyannote-style speaker counting. For v0.1 this
//! produces a descriptor but always emits 1 (single speaker) so scenes that
//! reference `audio.speaker_count` compile.

use perceptkit_core::{FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow};

use crate::extractor::FeatureExtractor;

/// Stub multi-speaker extractor. Always emits `audio.speaker_count = 1`.
#[derive(Debug, Default, Clone)]
pub struct MultiSpeakerExtractor;

impl MultiSpeakerExtractor {
    /// Construct.
    pub fn new() -> Self {
        Self
    }
}

impl FeatureExtractor for MultiSpeakerExtractor {
    fn name(&self) -> &'static str {
        "perceptkit-audio::MultiSpeakerExtractor (stub)"
    }

    fn descriptors(&self) -> Vec<FeatureDescriptor> {
        vec![FeatureDescriptor {
            key: FeatureKey::new("audio.speaker_count").unwrap(),
            kind: FeatureKind::F64 {
                min: Some(0.0),
                max: Some(16.0),
            },
            unit: Some("count".into()),
            window: TimeWindow::Sliding { ms: 5000 },
            source: concat!("perceptkit-audio@", env!("CARGO_PKG_VERSION"), "-stub").into(),
            version: 0,
        }]
    }

    fn extract(&self, _pcm: &[f32], _sample_rate: u32) -> Vec<(FeatureKey, FeatureValue)> {
        vec![(
            FeatureKey::new("audio.speaker_count").unwrap(),
            FeatureValue::F64(1.0),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_emits_one_speaker() {
        let out = MultiSpeakerExtractor.extract(&[0.0; 100], 16000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1.as_f64(), Some(1.0));
    }
}
