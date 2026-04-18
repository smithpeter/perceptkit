//! Confidence Gate — decides whether a SceneDecision is final or needs Cold Path.

use crate::matcher::arbiter::EvalCtx;
use crate::scene::SceneDecision;

/// Gate verdict — what to do with a hot-path decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateVerdict {
    /// Confident enough → emit decision as-is.
    Accept,
    /// Ambiguous / low-confidence → escalate to Cold Path Reflector.
    Escalate {
        /// Why the gate chose to escalate (for audit).
        reason: String,
    },
    /// Confidently no-match → emit `Unknown` without Cold Path escalation.
    Reject,
}

/// Gate strategy.
pub trait ConfidenceGate: Send + Sync {
    /// Classify a decision.
    fn verdict(&self, decision: &SceneDecision, ctx: &EvalCtx<'_>) -> GateVerdict;
}

/// Threshold-based gate:
/// - `confidence ≥ accept` → Accept
/// - `confidence ≤ reject_below` → Reject
/// - otherwise → Escalate
#[derive(Debug, Clone)]
pub struct ThresholdGate {
    /// Confidence at or above which to Accept.
    pub accept: f64,
    /// Confidence at or below which to Reject outright (no reflection).
    pub reject_below: f64,
}

impl Default for ThresholdGate {
    fn default() -> Self {
        Self {
            accept: 0.70,
            reject_below: 0.05,
        }
    }
}

impl ConfidenceGate for ThresholdGate {
    fn verdict(&self, decision: &SceneDecision, _ctx: &EvalCtx<'_>) -> GateVerdict {
        let c = decision.confidence;
        if !decision.is_known() || c <= self.reject_below {
            return GateVerdict::Reject;
        }
        if c >= self.accept {
            return GateVerdict::Accept;
        }
        GateVerdict::Escalate {
            reason: format!(
                "confidence {c:.2} in ({r:.2}, {a:.2}) — needs reflection",
                r = self.reject_below,
                a = self.accept
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::FeatureBundle;
    use crate::registry::FeatureRegistry;
    use crate::scene::{DecisionSource, SceneDecision};

    fn ctx<'a>(bundle: &'a FeatureBundle, reg: &'a FeatureRegistry) -> EvalCtx<'a> {
        EvalCtx {
            bundle,
            registry: reg,
            scenes: &[],
        }
    }

    fn decision(conf: f64, known: bool) -> SceneDecision {
        SceneDecision {
            scene_id: if known { Some("x".into()) } else { None },
            confidence: conf,
            description: None,
            source: DecisionSource::Rule,
            rationale: vec![],
        }
    }

    #[test]
    fn accept_above_threshold() {
        let g = ThresholdGate::default();
        let b = FeatureBundle::new(0.0);
        let r = FeatureRegistry::new();
        let c = ctx(&b, &r);
        assert_eq!(g.verdict(&decision(0.90, true), &c), GateVerdict::Accept);
    }

    #[test]
    fn reject_unknown() {
        let g = ThresholdGate::default();
        let b = FeatureBundle::new(0.0);
        let r = FeatureRegistry::new();
        let c = ctx(&b, &r);
        assert_eq!(g.verdict(&decision(0.0, false), &c), GateVerdict::Reject);
    }

    #[test]
    fn escalate_in_mid_range() {
        let g = ThresholdGate::default();
        let b = FeatureBundle::new(0.0);
        let r = FeatureRegistry::new();
        let c = ctx(&b, &r);
        let v = g.verdict(&decision(0.45, true), &c);
        matches!(v, GateVerdict::Escalate { .. });
    }
}
