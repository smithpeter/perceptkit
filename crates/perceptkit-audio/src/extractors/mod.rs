//! Feature extractors.

pub mod energy;
#[cfg(feature = "silero-vad")]
pub mod silero;
pub mod speaker;
pub mod spectral;
pub mod vad;
