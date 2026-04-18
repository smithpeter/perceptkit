//! Feature extractors.

pub mod energy;
#[cfg(feature = "silero-vad")]
pub mod silero;
pub mod speaker;
pub mod vad;
