//! Arbiter — combine candidate matches into a single SceneDecision.

use crate::feature::FeatureBundle;
use crate::matcher::rule::SceneMatch;
use crate::registry::FeatureRegistry;
use crate::scene::{DecisionSource, Evidence, EvidenceKind, Scene, SceneDecision};

/// Evaluation context passed through hot path.
pub struct EvalCtx<'a> {
    /// Feature bundle being evaluated.
    pub bundle: &'a FeatureBundle,
    /// Registered features (for provenance / unit awareness).
    pub registry: &'a FeatureRegistry,
    /// Full scene list (for describe template rendering).
    pub scenes: &'a [Scene],
}

/// Arbiter strategy — picks final decision from candidate matches.
pub trait Arbiter: Send + Sync {
    /// Produce the final SceneDecision.
    fn decide(&self, ctx: &EvalCtx<'_>, matches: &[SceneMatch]) -> SceneDecision;
}

/// Priority-then-confidence arbiter:
/// 1. Pick candidate(s) with highest `priority`.
/// 2. Among those, pick highest `confidence`.
/// 3. Tie-break by scene_id (alphabetical).
#[derive(Debug, Default, Clone)]
pub struct PriorityArbiter;

impl PriorityArbiter {
    /// Create an instance.
    pub fn new() -> Self {
        Self
    }
}

impl Arbiter for PriorityArbiter {
    fn decide(&self, ctx: &EvalCtx<'_>, matches: &[SceneMatch]) -> SceneDecision {
        if matches.is_empty() {
            return SceneDecision::unknown();
        }
        // Stable max by (priority desc, confidence desc, scene_id asc).
        let mut best: Option<&SceneMatch> = None;
        for m in matches {
            best = Some(match best {
                None => m,
                Some(cur) => {
                    let m_key = (m.priority, m.confidence);
                    let cur_key = (cur.priority, cur.confidence);
                    if m_key > cur_key || (m_key == cur_key && m.scene_id < cur.scene_id) {
                        m
                    } else {
                        cur
                    }
                }
            });
        }
        let best = best.expect("matches non-empty");

        let scene = ctx.scenes.iter().find(|s| s.id == best.scene_id);
        let description = scene.map(|s| render_template(&s.describe.template, ctx.bundle));

        let mut rationale = best.evidence.clone();
        rationale.push(Evidence {
            kind: EvidenceKind::RuleFired,
            description: format!(
                "arbiter chose '{}' (priority={}, confidence={:.2})",
                best.scene_id, best.priority, best.confidence
            ),
        });

        SceneDecision {
            scene_id: Some(best.scene_id.clone()),
            confidence: best.confidence,
            description,
            source: DecisionSource::Rule,
            rationale,
        }
    }
}

/// Very simple template renderer: leaves `{placeholders}` as-is in v0.1.
/// v0.2 will interpolate from Describe.fields mapping.
fn render_template(template: &str, _bundle: &FeatureBundle) -> String {
    template.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::rule::SceneMatch;

    fn make_match(id: &str, conf: f64, prio: i32) -> SceneMatch {
        SceneMatch {
            scene_id: id.into(),
            confidence: conf,
            evidence: vec![],
            priority: prio,
        }
    }

    #[test]
    fn empty_returns_unknown() {
        let reg = FeatureRegistry::new();
        let bundle = FeatureBundle::new(0.0);
        let ctx = EvalCtx {
            bundle: &bundle,
            registry: &reg,
            scenes: &[],
        };
        let decision = PriorityArbiter::new().decide(&ctx, &[]);
        assert!(!decision.is_known());
    }

    #[test]
    fn higher_priority_wins() {
        let reg = FeatureRegistry::new();
        let bundle = FeatureBundle::new(0.0);
        let ctx = EvalCtx {
            bundle: &bundle,
            registry: &reg,
            scenes: &[],
        };
        let matches = vec![
            make_match("low", 1.0, 5),
            make_match("high", 1.0, 20),
            make_match("mid", 1.0, 10),
        ];
        let decision = PriorityArbiter::new().decide(&ctx, &matches);
        assert_eq!(decision.scene_id.as_deref(), Some("high"));
    }

    #[test]
    fn same_priority_alphabetical_tie_break() {
        let reg = FeatureRegistry::new();
        let bundle = FeatureBundle::new(0.0);
        let ctx = EvalCtx {
            bundle: &bundle,
            registry: &reg,
            scenes: &[],
        };
        let matches = vec![make_match("banana", 1.0, 10), make_match("apple", 1.0, 10)];
        let decision = PriorityArbiter::new().decide(&ctx, &matches);
        assert_eq!(decision.scene_id.as_deref(), Some("apple"));
    }
}
