//! perceptkit-audio — Audio provider and feature extractors for perceptkit.
//!
//! v0.1 scope (M3):
//! - `EnergyExtractor`: RMS, peak, spectrum-subtraction SNR
//! - `VoiceActivityExtractor`: energy + zero-crossing rate
//! - `MultiSpeakerExtractor`: voice_ratio (trait stub in v0.1)
//! - `SoundEventExtractor`: trait definition (YAMNet/BEATs integration in v0.2)
//!
//! # v0.1 Scaffold
//!
//! This is the M1 scaffold. Real extractors land in M3.

#![forbid(unsafe_code)]

/// Version of `perceptkit-audio`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
