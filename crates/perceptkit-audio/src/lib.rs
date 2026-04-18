//! perceptkit-audio — Audio provider and feature extractors.
//!
//! v0.1 scope (M3):
//! - `EnergyExtractor`: RMS / peak / dBFS
//! - `VoiceActivityExtractor`: energy + zero-crossing rate → voice_activity / voice_ratio
//! - `MultiSpeakerExtractor`: **stub** (v0.2 integration)
//! - `SoundEventExtractor`: **trait only** (v0.2 YAMNet/BEATs integration)
//! - `AudioProvider`: composes extractors → `FeatureBundle`
//!
//! Signal Model: no network, no file I/O — caller provides decoded PCM.

#![forbid(unsafe_code)]

pub mod descriptors;
pub mod extractor;
pub mod extractors;
pub mod provider;

pub use descriptors::audio_descriptors;
pub use extractor::FeatureExtractor;
pub use extractors::energy::EnergyExtractor;
#[cfg(feature = "silero-vad")]
pub use extractors::silero::SileroVadExtractor;
pub use extractors::speaker::MultiSpeakerExtractor;
pub use extractors::vad::VoiceActivityExtractor;
pub use provider::AudioProvider;

/// Version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
