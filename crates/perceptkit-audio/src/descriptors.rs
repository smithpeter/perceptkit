//! Convenience: default `FeatureDescriptor` set for audio.*.
//!
//! Use when setting up `FeatureRegistry`:
//!
//! ```
//! # use perceptkit_core::FeatureRegistry;
//! # use perceptkit_audio::audio_descriptors;
//! let mut reg = FeatureRegistry::new();
//! for d in audio_descriptors() {
//!     reg.register(d);
//! }
//! ```

use perceptkit_core::FeatureDescriptor;

use crate::extractor::FeatureExtractor;
use crate::extractors::{
    energy::EnergyExtractor, speaker::MultiSpeakerExtractor, spectral::SpectralExtractor,
    vad::VoiceActivityExtractor,
};

/// Default descriptors produced by the audio crate's extractors.
pub fn audio_descriptors() -> Vec<FeatureDescriptor> {
    let mut out = Vec::new();
    out.extend(EnergyExtractor.descriptors());
    out.extend(VoiceActivityExtractor::default().descriptors());
    out.extend(MultiSpeakerExtractor.descriptors());
    out.extend(SpectralExtractor::default().descriptors());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_descriptors_nonempty() {
        let d = audio_descriptors();
        assert!(d.len() >= 7);
    }

    #[test]
    fn all_descriptors_have_unique_keys() {
        let d = audio_descriptors();
        let keys: std::collections::HashSet<_> =
            d.iter().map(|x| x.key.as_str().to_string()).collect();
        assert_eq!(keys.len(), d.len());
    }
}
