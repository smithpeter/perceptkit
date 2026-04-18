//! `AudioProvider` — composes extractors into a `FeatureBundle`.
//!
//! Usage:
//! ```no_run
//! # use perceptkit_audio::AudioProvider;
//! let provider = AudioProvider::with_defaults();
//! let pcm = vec![0.0_f32; 16000];
//! let bundle = provider.process(&pcm, 16000, 0.0);
//! ```

use perceptkit_core::FeatureBundle;

use crate::extractor::FeatureExtractor;
use crate::extractors::{
    energy::EnergyExtractor, speaker::MultiSpeakerExtractor, spectral::SpectralExtractor,
    vad::VoiceActivityExtractor,
};

/// Orchestrates extractors → `FeatureBundle` per tick.
pub struct AudioProvider {
    extractors: Vec<Box<dyn FeatureExtractor>>,
}

impl AudioProvider {
    /// Construct with no extractors.
    pub fn new() -> Self {
        Self {
            extractors: Vec::new(),
        }
    }

    /// Construct with the default extractor set:
    /// Energy + VoiceActivity + MultiSpeaker (stub) + Spectral.
    pub fn with_defaults() -> Self {
        Self {
            extractors: vec![
                Box::new(EnergyExtractor),
                Box::new(VoiceActivityExtractor::default()),
                Box::new(MultiSpeakerExtractor),
                Box::new(SpectralExtractor::default()),
            ],
        }
    }

    /// Append a custom extractor.
    pub fn with_extractor(mut self, extractor: Box<dyn FeatureExtractor>) -> Self {
        self.extractors.push(extractor);
        self
    }

    /// Number of registered extractors.
    pub fn len(&self) -> usize {
        self.extractors.len()
    }

    /// Whether the provider has no extractors.
    pub fn is_empty(&self) -> bool {
        self.extractors.is_empty()
    }

    /// Names of installed extractors (for audit).
    pub fn extractor_names(&self) -> Vec<&'static str> {
        self.extractors.iter().map(|e| e.name()).collect()
    }

    /// Process a PCM buffer → `FeatureBundle`.
    ///
    /// - `pcm`: f32 mono samples in `[-1.0, 1.0]`.
    /// - `sample_rate`: samples per second (typically 16000).
    /// - `timestamp`: unix seconds — attached to the bundle.
    pub fn process(&self, pcm: &[f32], sample_rate: u32, timestamp: f64) -> FeatureBundle {
        let mut bundle = FeatureBundle::new(timestamp);
        for ex in &self.extractors {
            bundle.sources.push(ex.name().to_string());
            for (k, v) in ex.extract(pcm, sample_rate) {
                bundle.insert(k, v);
            }
        }
        bundle
    }
}

impl Default for AudioProvider {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_populate_expected_features() {
        let p = AudioProvider::with_defaults();
        let pcm = vec![0.5_f32; 16000];
        let b = p.process(&pcm, 16000, 12345.6);
        assert_eq!(b.timestamp, 12345.6);
        // Energy (4) + VAD (3) + Speaker (1) + Spectral (3) = 11
        assert_eq!(b.len(), 11);
        assert!(b.get_str("audio.rms").is_some());
        assert!(b.get_str("audio.voice_activity").is_some());
        assert!(b.get_str("audio.speaker_count").is_some());
    }

    #[test]
    fn empty_provider_yields_empty_bundle() {
        let p = AudioProvider::new();
        let b = p.process(&[0.0; 100], 16000, 0.0);
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn sources_recorded() {
        let p = AudioProvider::with_defaults();
        let b = p.process(&[0.0; 100], 16000, 0.0);
        // 4 default extractors: Energy, VAD, Speaker, Spectral
        assert_eq!(b.sources.len(), 4);
    }
}
