//! Cold Path Reflector — LLM tool-calling agent for uncertain scenes.

pub mod noop;

pub use noop::NoopReflector;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::feature::FeatureBundle;
use crate::scene::SceneDecision;

/// Budget enforcement for reflection calls — never hang.
#[derive(Debug, Clone, Copy)]
pub struct ReflectionBudget {
    /// Max wall-clock time in milliseconds.
    pub max_time_ms: u64,
    /// Max tokens (for LLM-backed reflectors).
    pub max_tokens: u32,
    /// Max tool calls allowed per reflection.
    pub max_tool_calls: u32,
}

impl Default for ReflectionBudget {
    fn default() -> Self {
        Self {
            max_time_ms: 2000,
            max_tokens: 1024,
            max_tool_calls: 7,
        }
    }
}

/// A pending case — hot-path produced a low-confidence decision, escalated here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCase {
    /// Unique case id (ULID / UUID).
    pub id: String,
    /// Timestamp (unix seconds).
    pub timestamp: f64,
    /// Feature bundle keys/values at the time (serialized).
    pub features: Vec<(String, serde_yml::Value)>,
    /// Reason for escalation from the gate.
    pub reason: String,
    /// The decision that was escalated.
    pub failed_decision: SceneDecision,
}

impl PendingCase {
    /// Build a pending case from hot-path components.
    pub fn from_bundle(
        id: String,
        bundle: &FeatureBundle,
        reason: String,
        failed_decision: SceneDecision,
    ) -> Self {
        let features: Vec<(String, serde_yml::Value)> = bundle
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    serde_yml::to_value(v).unwrap_or(serde_yml::Value::Null),
                )
            })
            .collect();
        Self {
            id,
            timestamp: bundle.timestamp,
            features,
            reason,
            failed_decision,
        }
    }
}

/// Outcome of a Reflector reflection — three-way output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reflection {
    /// Map this case onto an existing scene.
    Map {
        /// Known scene id the case maps to.
        scene_id: String,
        /// LLM-provided rationale.
        rationale: String,
    },
    /// Propose a brand-new scene (enters PendingSceneQueue for human review).
    Propose {
        /// Proposed YAML of the new scene.
        yaml: String,
        /// Example feature bundles supporting this proposal.
        examples: Vec<String>,
    },
    /// Honest "I don't know" — emit with rich metadata.
    Unknown {
        /// Natural-language summary of features.
        summary: String,
        /// Top-weighted features (for debug).
        top_features: Vec<String>,
    },
}

impl Reflection {
    /// Construct an `Unknown` reflection from a brief summary.
    pub fn unknown(summary: impl Into<String>) -> Self {
        Self::Unknown {
            summary: summary.into(),
            top_features: Vec::new(),
        }
    }
}

/// Stable fingerprint for prompt/program version — enables snapshot tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptHash(pub String);

/// Reflection errors.
#[derive(Debug, thiserror::Error)]
pub enum ReflectError {
    /// Budget exceeded (time / tokens / tool calls).
    #[error("reflection budget exceeded: {0}")]
    Budget(String),
    /// Backend (LLM / network / model) error.
    #[error("reflector backend error: {0}")]
    Backend(String),
    /// Produced YAML was invalid (schema).
    #[error("proposed YAML invalid: {0}")]
    InvalidProposal(String),
    /// Upstream conversion from core Error.
    #[error(transparent)]
    Core(#[from] Error),
}

/// Async Reflector trait. Implementations: `NoopReflector` (v0.1),
/// `MockReflector` (M6 test fixture), `LocalReflector` (M6 Qwen-0.5B).
#[async_trait]
pub trait Reflector: Send + Sync {
    /// Reflect on a pending case.
    async fn reflect(&self, case: PendingCase) -> Result<Reflection, ReflectError>;

    /// Human-readable backend name (for audit).
    fn name(&self) -> &'static str;

    /// Stable fingerprint for this reflector version.
    fn fingerprint(&self) -> PromptHash;

    /// Budget this reflector enforces.
    fn budget(&self) -> ReflectionBudget {
        ReflectionBudget::default()
    }
}
