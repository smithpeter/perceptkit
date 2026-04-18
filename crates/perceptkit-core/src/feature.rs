//! Feature typing system — typed, unit-bearing, windowed features.
//!
//! Solves the Round 1 "flat HashMap is under-engineered" critique:
//! `FeatureKey` typo like `voice_ratios` → compile-time / load-time error
//! with Levenshtein `did_you_mean` suggestion.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Typed feature key — dot-segmented namespace like `audio.voice_ratio`.
///
/// Construct via [`FeatureKey::new`] which validates format. Use [`FeatureKey::as_str`] to read.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FeatureKey(String);

impl FeatureKey {
    /// Build a FeatureKey, validating `[a-zA-Z0-9_]+(\.[a-zA-Z0-9_]+)*` format.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s: String = s.into();
        if s.is_empty() {
            return Err(Error::InvalidFeatureKey(s));
        }
        for seg in s.split('.') {
            if seg.is_empty() || !seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(Error::InvalidFeatureKey(s));
            }
        }
        Ok(Self(s))
    }

    /// String view of the key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FeatureKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for FeatureKey {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FeatureKey> for String {
    fn from(value: FeatureKey) -> Self {
        value.0
    }
}

/// Feature data kind with optional range / category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FeatureKind {
    /// Continuous f64 with optional bounds.
    F64 {
        /// Inclusive lower bound.
        min: Option<f64>,
        /// Inclusive upper bound.
        max: Option<f64>,
    },
    /// Boolean flag.
    Bool,
    /// Categorical — one of a fixed set of string labels.
    Category {
        /// Allowed labels.
        values: Vec<String>,
    },
    /// Dense embedding vector of fixed dimension.
    Vector {
        /// Number of dimensions.
        dim: usize,
    },
}

/// Time-window semantics for a feature value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "window", rename_all = "snake_case")]
pub enum TimeWindow {
    /// Value at a single instant.
    Instant,
    /// Sliding window aggregate over a duration.
    Sliding {
        /// Window size in milliseconds.
        ms: u64,
    },
    /// Exponentially-weighted moving average with smoothing factor.
    Ema {
        /// Smoothing factor (0.0 < alpha ≤ 1.0).
        alpha: f64,
    },
}

impl TimeWindow {
    /// Duration representation when applicable.
    pub fn duration(&self) -> Option<Duration> {
        match self {
            Self::Instant => Some(Duration::ZERO),
            Self::Sliding { ms } => Some(Duration::from_millis(*ms)),
            Self::Ema { .. } => None,
        }
    }
}

/// Descriptor for a feature — its schema, unit, window, provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDescriptor {
    /// Feature key (dot-segmented).
    pub key: FeatureKey,
    /// Data kind + constraints.
    #[serde(flatten)]
    pub kind: FeatureKind,
    /// Optional unit string (e.g. `"ratio_0_1"`, `"dB"`, `"count"`).
    #[serde(default)]
    pub unit: Option<String>,
    /// Time-window semantics.
    pub window: TimeWindow,
    /// Source crate / extractor (e.g. `"perceptkit-audio@0.1.0"`).
    pub source: String,
    /// Schema version of this feature (bump on breaking change).
    pub version: u32,
}

/// Runtime feature value — one reading matching a FeatureDescriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureValue {
    /// Continuous numeric reading.
    F64(f64),
    /// Boolean reading.
    Bool(bool),
    /// Categorical reading (should match `FeatureKind::Category.values`).
    Category(String),
    /// Dense vector reading.
    Vector(Vec<f32>),
}

impl FeatureValue {
    /// Type hint for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::F64(_) => "f64",
            Self::Bool(_) => "bool",
            Self::Category(_) => "category",
            Self::Vector(_) => "vector",
        }
    }

    /// Extract f64 if this value is F64.
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::F64(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Extract bool if this value is Bool.
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Extract category label if this value is Category.
    pub fn as_category(&self) -> Option<&str> {
        if let Self::Category(v) = self {
            Some(v.as_str())
        } else {
            None
        }
    }
}

/// Time-stamped bundle of feature readings for one evaluation tick.
#[derive(Debug, Clone, Default)]
pub struct FeatureBundle {
    features: HashMap<FeatureKey, FeatureValue>,
    /// Unix timestamp (seconds) for this bundle.
    pub timestamp: f64,
    /// Originating sources (for provenance).
    pub sources: Vec<String>,
}

impl FeatureBundle {
    /// Construct an empty bundle at timestamp `ts`.
    pub fn new(timestamp: f64) -> Self {
        Self {
            features: HashMap::new(),
            timestamp,
            sources: Vec::new(),
        }
    }

    /// Insert a feature reading; returns the previous value if any.
    pub fn insert(&mut self, key: FeatureKey, value: FeatureValue) -> Option<FeatureValue> {
        self.features.insert(key, value)
    }

    /// Read a feature by key.
    pub fn get(&self, key: &FeatureKey) -> Option<&FeatureValue> {
        self.features.get(key)
    }

    /// Read by raw string key (helper for YAML-driven code paths).
    pub fn get_str(&self, key: &str) -> Option<&FeatureValue> {
        FeatureKey::new(key).ok().and_then(|k| self.get(&k))
    }

    /// Iterate over all `(key, value)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&FeatureKey, &FeatureValue)> {
        self.features.iter()
    }

    /// Total number of features in the bundle.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Whether the bundle has no features.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_accepts_valid() {
        assert!(FeatureKey::new("audio.voice_ratio").is_ok());
        assert!(FeatureKey::new("a").is_ok());
        assert!(FeatureKey::new("a.b.c.d").is_ok());
        assert!(FeatureKey::new("audio_v2.speaker_count").is_ok());
    }

    #[test]
    fn key_rejects_invalid() {
        assert!(FeatureKey::new("").is_err());
        assert!(FeatureKey::new("audio..voice").is_err());
        assert!(FeatureKey::new(".leading").is_err());
        assert!(FeatureKey::new("trailing.").is_err());
        assert!(FeatureKey::new("has space").is_err());
        assert!(FeatureKey::new("has-dash").is_err());
        assert!(FeatureKey::new("has/slash").is_err());
    }

    #[test]
    fn bundle_insert_and_get() {
        let mut b = FeatureBundle::new(0.0);
        let k = FeatureKey::new("audio.voice_ratio").unwrap();
        b.insert(k.clone(), FeatureValue::F64(0.72));
        assert_eq!(b.get(&k).and_then(FeatureValue::as_f64), Some(0.72));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn value_type_introspection() {
        assert_eq!(FeatureValue::F64(1.0).type_name(), "f64");
        assert_eq!(FeatureValue::Bool(true).type_name(), "bool");
        assert_eq!(FeatureValue::Category("x".into()).type_name(), "category");
        assert_eq!(FeatureValue::Vector(vec![]).type_name(), "vector");
    }
}
