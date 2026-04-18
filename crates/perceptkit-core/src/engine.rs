//! SceneEngine — top-level orchestrator wiring Loader + Matcher + Arbiter + Gate.

use std::path::Path;
use std::sync::Arc;

use crate::dsl::loader::load_dir;
use crate::error::Result;
use crate::feature::FeatureBundle;
use crate::gate::{ConfidenceGate, GateVerdict, ThresholdGate};
use crate::matcher::arbiter::{Arbiter, EvalCtx, PriorityArbiter};
use crate::matcher::rule::{RuleMatcher, SimpleRuleMatcher};
use crate::reflector::{NoopReflector, Reflector};
use crate::registry::FeatureRegistry;
use crate::scene::{DecisionSource, Scene, SceneDecision};

/// Lint report from `SceneEngine::lint`.
#[derive(Debug, Default)]
pub struct LintReport {
    /// Scenes loaded successfully.
    pub scenes_ok: usize,
    /// Detected scene-id collisions or overlaps.
    pub conflicts: Vec<LintConflict>,
    /// Informational warnings.
    pub warnings: Vec<String>,
}

impl LintReport {
    /// Whether the lint passed (no conflicts).
    pub fn passed(&self) -> bool {
        self.conflicts.is_empty()
    }
}

/// A detected conflict between two scenes (e.g. overlapping rules, no priority tie-break).
#[derive(Debug, Clone)]
pub struct LintConflict {
    /// First scene id.
    pub scene_a: String,
    /// Second scene id.
    pub scene_b: String,
    /// Human-readable explanation.
    pub reason: String,
}

/// Top-level SceneEngine. Not yet streaming — v0.1 is request/response.
pub struct SceneEngine {
    scenes: Vec<Scene>,
    registry: FeatureRegistry,
    matcher: Box<dyn RuleMatcher>,
    arbiter: Box<dyn Arbiter>,
    gate: Box<dyn ConfidenceGate>,
    reflector: Arc<dyn Reflector>,
}

impl SceneEngine {
    /// Load scenes from directory, wire default components (SimpleRuleMatcher,
    /// PriorityArbiter, ThresholdGate, NoopReflector).
    pub fn from_dir(path: &Path) -> Result<Self> {
        let registry = FeatureRegistry::new();
        let scenes = load_dir(path, &registry)?;
        Ok(Self {
            scenes,
            registry,
            matcher: Box::new(SimpleRuleMatcher::new()),
            arbiter: Box::new(PriorityArbiter::new()),
            gate: Box::new(ThresholdGate::default()),
            reflector: Arc::new(NoopReflector::new()),
        })
    }

    /// Override the feature registry (providers call this to register descriptors).
    pub fn with_registry(mut self, registry: FeatureRegistry) -> Self {
        self.registry = registry;
        self
    }

    /// Override the reflector (default is NoopReflector).
    pub fn with_reflector(mut self, reflector: Arc<dyn Reflector>) -> Self {
        self.reflector = reflector;
        self
    }

    /// Override the gate.
    pub fn with_gate(mut self, gate: Box<dyn ConfidenceGate>) -> Self {
        self.gate = gate;
        self
    }

    /// Access loaded scenes (for inspection).
    pub fn scenes(&self) -> &[Scene] {
        &self.scenes
    }

    /// Access the feature registry.
    pub fn registry(&self) -> &FeatureRegistry {
        &self.registry
    }

    /// Evaluate a FeatureBundle through the Hot Path only (sync).
    ///
    /// Returns `SceneDecision` with `source = Rule` for accepted decisions,
    /// `Fallback` for rejected/unknown. Cold Path requires `.evaluate_async()`.
    pub fn evaluate(&self, bundle: &FeatureBundle) -> SceneDecision {
        let ctx = EvalCtx {
            bundle,
            registry: &self.registry,
            scenes: &self.scenes,
        };
        let matches = self.matcher.match_scenes(bundle, &self.scenes);
        let decision = self.arbiter.decide(&ctx, &matches);

        match self.gate.verdict(&decision, &ctx) {
            GateVerdict::Accept => decision,
            GateVerdict::Reject => SceneDecision::unknown(),
            GateVerdict::Escalate { reason } => {
                // Hot-path-only: no reflector call; annotate and return as-is.
                let mut d = decision;
                d.source = DecisionSource::Fallback;
                d.rationale.push(crate::scene::Evidence {
                    kind: crate::scene::EvidenceKind::Reflection,
                    description: format!(
                        "gate escalated (reason: {reason}) — sync evaluate() skipped Cold Path"
                    ),
                });
                d
            }
        }
    }

    /// Evaluate through Hot Path + optionally Cold Path (async).
    pub async fn evaluate_async(&self, bundle: &FeatureBundle) -> SceneDecision {
        let ctx = EvalCtx {
            bundle,
            registry: &self.registry,
            scenes: &self.scenes,
        };
        let matches = self.matcher.match_scenes(bundle, &self.scenes);
        let decision = self.arbiter.decide(&ctx, &matches);

        match self.gate.verdict(&decision, &ctx) {
            GateVerdict::Accept => decision,
            GateVerdict::Reject => SceneDecision::unknown(),
            GateVerdict::Escalate { reason } => {
                let case = crate::reflector::PendingCase::from_bundle(
                    format!("case-{}", (bundle.timestamp * 1000.0) as u64),
                    bundle,
                    reason,
                    decision.clone(),
                );
                match self.reflector.reflect(case).await {
                    Ok(refl) => reflection_to_decision(refl, decision),
                    Err(_) => {
                        // Reflector failed → honest Unknown
                        SceneDecision::unknown()
                    }
                }
            }
        }
    }

    /// Lint all scenes — checks for id duplicates (already caught at load),
    /// priority conflicts, etc. v0.1 is minimal; v0.2 adds semantic overlap.
    pub fn lint(path: &Path) -> Result<LintReport> {
        let registry = FeatureRegistry::new();
        let scenes = load_dir(path, &registry)?;
        let mut report = LintReport {
            scenes_ok: scenes.len(),
            ..Default::default()
        };

        // Check for scenes with identical priority + overlapping `all` conditions.
        // v0.1 heuristic: two scenes with same priority that share any `all`
        // feature-key are flagged as "potential conflict".
        for (i, a) in scenes.iter().enumerate() {
            for b in scenes.iter().skip(i + 1) {
                if a.priority != b.priority {
                    continue;
                }
                let a_keys: std::collections::HashSet<&str> = a
                    .match_rules
                    .all
                    .iter()
                    .map(|c| c.feature.as_str())
                    .collect();
                let b_keys: std::collections::HashSet<&str> = b
                    .match_rules
                    .all
                    .iter()
                    .map(|c| c.feature.as_str())
                    .collect();
                if !a_keys.is_disjoint(&b_keys) {
                    report.conflicts.push(LintConflict {
                        scene_a: a.id.clone(),
                        scene_b: b.id.clone(),
                        reason: format!(
                            "same priority {} and overlapping `all` feature keys",
                            a.priority
                        ),
                    });
                }
            }
        }

        Ok(report)
    }
}

fn reflection_to_decision(
    reflection: crate::reflector::Reflection,
    fallback: SceneDecision,
) -> SceneDecision {
    use crate::reflector::Reflection;
    match reflection {
        Reflection::Map {
            scene_id,
            rationale,
        } => SceneDecision {
            scene_id: Some(scene_id),
            confidence: fallback.confidence.max(0.60),
            description: None,
            source: DecisionSource::Reflection,
            rationale: vec![crate::scene::Evidence {
                kind: crate::scene::EvidenceKind::Reflection,
                description: rationale,
            }],
        },
        Reflection::Propose { .. } => {
            // v0.1: just emit Unknown with note; M6 wires PendingSceneQueue push.
            let mut d = SceneDecision::unknown();
            d.rationale.push(crate::scene::Evidence {
                kind: crate::scene::EvidenceKind::Reflection,
                description: "reflector proposed new scene (queued for review)".into(),
            });
            d
        }
        Reflection::Unknown { summary, .. } => {
            let mut d = SceneDecision::unknown();
            d.rationale.push(crate::scene::Evidence {
                kind: crate::scene::EvidenceKind::Reflection,
                description: summary,
            });
            d
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::FeatureKey;
    use crate::feature::FeatureValue;

    fn write(dir: &Path, name: &str, yaml: &str) {
        std::fs::write(dir.join(name), yaml).unwrap();
    }

    #[test]
    fn from_dir_loads_scenes() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "x.yaml",
            r#"
id: x
version: 1
describe: {template: "x"}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
priority: 5
"#,
        );
        let engine = SceneEngine::from_dir(tmp.path()).unwrap();
        assert_eq!(engine.scenes().len(), 1);
    }

    #[test]
    fn evaluate_matches_scene() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "meeting.yaml",
            r#"
id: meeting
version: 1
describe: {template: "meeting"}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
priority: 10
"#,
        );
        let engine = SceneEngine::from_dir(tmp.path()).unwrap();
        let mut b = FeatureBundle::new(0.0);
        b.insert(
            FeatureKey::new("audio.voice_ratio").unwrap(),
            FeatureValue::F64(0.80),
        );
        let d = engine.evaluate(&b);
        assert_eq!(d.scene_id.as_deref(), Some("meeting"));
    }

    #[test]
    fn evaluate_empty_bundle_returns_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "x.yaml",
            r#"
id: x
version: 1
describe: {template: "x"}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
"#,
        );
        let engine = SceneEngine::from_dir(tmp.path()).unwrap();
        let b = FeatureBundle::new(0.0);
        let d = engine.evaluate(&b);
        assert!(!d.is_known());
    }

    #[test]
    fn lint_detects_overlapping_priority_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "a.yaml",
            r#"
id: a
version: 1
describe: {template: a}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
priority: 10
"#,
        );
        write(
            tmp.path(),
            "b.yaml",
            r#"
id: b
version: 1
describe: {template: b}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.7 }
priority: 10
"#,
        );
        let report = SceneEngine::lint(tmp.path()).unwrap();
        assert_eq!(report.scenes_ok, 2);
        assert_eq!(report.conflicts.len(), 1);
    }

    #[tokio::test]
    async fn async_evaluate_falls_back_to_noop_reflector() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "x.yaml",
            r#"
id: x
version: 1
describe: {template: x}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
"#,
        );
        let engine = SceneEngine::from_dir(tmp.path()).unwrap();
        let b = FeatureBundle::new(0.0);
        let d = engine.evaluate_async(&b).await;
        assert!(!d.is_known());
    }
}
