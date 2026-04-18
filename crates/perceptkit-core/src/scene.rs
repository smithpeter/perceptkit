//! Scene, SceneDecision, Evidence — domain types for perceptkit output.

use serde::{Deserialize, Serialize};

use crate::dsl::schema::{Describe, MatchRules};

/// A compiled scene (loaded from YAML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Unique scene id (e.g. `"online_meeting"`).
    pub id: String,
    /// Schema version (bump on breaking match rule changes).
    pub version: u32,
    /// Human-readable description template.
    pub describe: Describe,
    /// Match conditions (all/any/none).
    #[serde(rename = "match")]
    pub match_rules: MatchRules,
    /// Priority for tie-breaking (higher wins).
    #[serde(default)]
    pub priority: i32,
}

/// Where a `SceneDecision` came from — hot path rule / embedding / cold path reflection / fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    /// Matched by rule engine (hot path).
    Rule,
    /// Matched by embedding similarity (v0.2+).
    Embedding,
    /// Produced by LLM reflection (cold path).
    Reflection,
    /// Fallback when no other source produced a confident decision.
    Fallback,
}

/// Single piece of evidence supporting a scene decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// What kind of evidence this is.
    pub kind: EvidenceKind,
    /// Human-readable description.
    pub description: String,
}

/// Kind of evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A rule condition fired.
    RuleFired,
    /// A feature value contributed.
    FeatureValue,
    /// LLM reflection step.
    Reflection,
}

/// Final decision output for one evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDecision {
    /// Identified scene; `None` for `Unknown`.
    pub scene_id: Option<String>,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Rendered description (from Scene.describe template).
    pub description: Option<String>,
    /// Origin of this decision.
    pub source: DecisionSource,
    /// Supporting evidence (audit trail).
    pub rationale: Vec<Evidence>,
}

impl SceneDecision {
    /// Construct an `Unknown` decision (no scene matched).
    pub fn unknown() -> Self {
        Self {
            scene_id: None,
            confidence: 0.0,
            description: None,
            source: DecisionSource::Fallback,
            rationale: Vec::new(),
        }
    }

    /// Whether this decision identified a scene.
    pub fn is_known(&self) -> bool {
        self.scene_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_decision() {
        let d = SceneDecision::unknown();
        assert!(!d.is_known());
        assert_eq!(d.source, DecisionSource::Fallback);
    }
}
