//! Signal and Modality — provenance metadata for raw inputs.
//!
//! In v0.1, `Signal` is lightweight metadata. Feature extraction happens
//! in Provider crates (e.g. `perceptkit-audio`). The core engine consumes
//! `FeatureBundle` directly.

use serde::{Deserialize, Serialize};

/// Signal modality — which sensory channel an input originates from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    /// Audio (PCM / features).
    Audio,
    /// Visual (image / video frames) — v0.3+.
    Visual,
    /// Context (window / app / time / location / motion).
    Context,
    /// Text (user utterance / chat log) — v0.4+.
    Text,
}

/// Signal — provenance metadata wrapping an extracted feature event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Source identifier (e.g. `"perceptkit-audio@0.1.0"`).
    pub source: String,
    /// Unix timestamp (seconds since epoch).
    pub timestamp: f64,
    /// Modality classification.
    pub modality: Modality,
}

impl Signal {
    /// Construct a Signal with current wall-clock time (UTC seconds).
    pub fn now(source: impl Into<String>, modality: Modality) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        Self {
            source: source.into(),
            timestamp: ts,
            modality,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modality_serde_roundtrip() {
        let m = Modality::Audio;
        let s = serde_yml::to_string(&m).unwrap();
        assert!(s.contains("audio"));
        let back: Modality = serde_yml::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn signal_now_has_nonzero_timestamp() {
        let s = Signal::now("test", Modality::Audio);
        assert!(s.timestamp > 0.0);
        assert_eq!(s.modality, Modality::Audio);
    }
}
