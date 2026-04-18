//! Rule matcher — evaluates Scene YAML conditions against a FeatureBundle.

use crate::dsl::schema::{Condition, Op, Value};
use crate::feature::{FeatureBundle, FeatureValue};
use crate::scene::{Evidence, EvidenceKind, Scene};

/// Trait for hot-path rule matchers.
pub trait RuleMatcher: Send + Sync {
    /// Evaluate all scenes against a bundle, return candidate matches.
    fn match_scenes(&self, bundle: &FeatureBundle, scenes: &[Scene]) -> Vec<SceneMatch>;
}

/// Candidate match — one scene's evaluation result against a bundle.
#[derive(Debug, Clone)]
pub struct SceneMatch {
    /// Which scene matched.
    pub scene_id: String,
    /// Confidence in `[0.0, 1.0]` — fraction of conditions satisfied.
    pub confidence: f64,
    /// Human-readable evidence entries.
    pub evidence: Vec<Evidence>,
    /// Priority copied from the Scene (for Arbiter tie-breaking).
    pub priority: i32,
}

/// Simple boolean rule matcher: `all` must all pass, `any` must have ≥1 pass,
/// `none` must have all fail.
///
/// Confidence = 1.0 when scene fully matches, 0.0 otherwise. More nuanced
/// scoring (partial / weighted) is planned for v0.2.
#[derive(Debug, Default, Clone)]
pub struct SimpleRuleMatcher;

impl SimpleRuleMatcher {
    /// Create an instance.
    pub fn new() -> Self {
        Self
    }

    fn eval_condition(&self, cond: &Condition, bundle: &FeatureBundle) -> (bool, Evidence) {
        let Some(v) = bundle.get_str(&cond.feature) else {
            return (
                false,
                Evidence {
                    kind: EvidenceKind::FeatureValue,
                    description: format!("missing feature '{}'", cond.feature),
                },
            );
        };
        let passed = eval(cond.op, v, &cond.value);
        let evidence = Evidence {
            kind: EvidenceKind::RuleFired,
            description: format!(
                "{} {:?} {} → {}",
                cond.feature,
                cond.op,
                describe_value(&cond.value),
                if passed { "pass" } else { "fail" }
            ),
        };
        (passed, evidence)
    }

    fn match_one(&self, scene: &Scene, bundle: &FeatureBundle) -> SceneMatch {
        let mut evidence = Vec::new();

        // `all`: every condition must pass.
        let mut all_ok = true;
        for c in &scene.match_rules.all {
            let (ok, e) = self.eval_condition(c, bundle);
            evidence.push(e);
            if !ok {
                all_ok = false;
            }
        }

        // `any`: at least one must pass (if non-empty).
        let any_ok = if scene.match_rules.any.is_empty() {
            true
        } else {
            let mut seen_pass = false;
            for c in &scene.match_rules.any {
                let (ok, e) = self.eval_condition(c, bundle);
                evidence.push(e);
                if ok {
                    seen_pass = true;
                }
            }
            seen_pass
        };

        // `none`: all must fail.
        let mut none_ok = true;
        for c in &scene.match_rules.none {
            let (ok, e) = self.eval_condition(c, bundle);
            evidence.push(e);
            if ok {
                none_ok = false;
            }
        }

        let passed = all_ok && any_ok && none_ok;
        SceneMatch {
            scene_id: scene.id.clone(),
            confidence: if passed { 1.0 } else { 0.0 },
            evidence,
            priority: scene.priority,
        }
    }
}

impl RuleMatcher for SimpleRuleMatcher {
    fn match_scenes(&self, bundle: &FeatureBundle, scenes: &[Scene]) -> Vec<SceneMatch> {
        scenes
            .iter()
            .map(|s| self.match_one(s, bundle))
            .filter(|m| m.confidence > 0.0)
            .collect()
    }
}

fn eval(op: Op, value: &FeatureValue, target: &Value) -> bool {
    match op {
        Op::Gt => match (value.as_f64(), target.as_f64()) {
            (Some(a), Some(b)) => a > b,
            _ => false,
        },
        Op::Gte => match (value.as_f64(), target.as_f64()) {
            (Some(a), Some(b)) => a >= b,
            _ => false,
        },
        Op::Lt => match (value.as_f64(), target.as_f64()) {
            (Some(a), Some(b)) => a < b,
            _ => false,
        },
        Op::Lte => match (value.as_f64(), target.as_f64()) {
            (Some(a), Some(b)) => a <= b,
            _ => false,
        },
        Op::Eq => eq(value, target),
        Op::Ne => !eq(value, target),
        Op::In => match target.as_list() {
            Some(list) => list.iter().any(|v| eq(value, v)),
            None => false,
        },
        Op::NotIn => match target.as_list() {
            Some(list) => !list.iter().any(|v| eq(value, v)),
            None => false,
        },
    }
}

fn eq(value: &FeatureValue, target: &Value) -> bool {
    match (value, target) {
        (FeatureValue::F64(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
        (FeatureValue::Bool(a), Value::Bool(b)) => a == b,
        (FeatureValue::Category(a), Value::String(b)) => a == b,
        _ => false,
    }
}

fn describe_value(v: &Value) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::List(l) => format!("[{} items]", l.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::FeatureKey;
    use crate::scene::Scene;

    fn scene_yaml(yaml: &str) -> Scene {
        serde_yml::from_str(yaml).unwrap()
    }

    #[test]
    fn gt_passes_when_feature_greater() {
        let s = scene_yaml(
            r#"
id: loud
version: 1
describe: {template: loud}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
"#,
        );
        let mut b = FeatureBundle::new(0.0);
        b.insert(
            FeatureKey::new("audio.voice_ratio").unwrap(),
            FeatureValue::F64(0.72),
        );
        let matches = SimpleRuleMatcher::new().match_scenes(&b, &[s]);
        assert_eq!(matches.len(), 1);
        assert!((matches[0].confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn missing_feature_fails_all() {
        let s = scene_yaml(
            r#"
id: x
version: 1
describe: {template: x}
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
"#,
        );
        let b = FeatureBundle::new(0.0);
        let matches = SimpleRuleMatcher::new().match_scenes(&b, &[s]);
        assert!(matches.is_empty());
    }

    #[test]
    fn any_passes_with_one_match() {
        let s = scene_yaml(
            r#"
id: meeting
version: 1
describe: {template: meeting}
match:
  any:
    - { feature: context.app, op: eq, value: Zoom }
    - { feature: context.app, op: eq, value: Teams }
"#,
        );
        let mut b = FeatureBundle::new(0.0);
        b.insert(
            FeatureKey::new("context.app").unwrap(),
            FeatureValue::Category("Teams".into()),
        );
        let matches = SimpleRuleMatcher::new().match_scenes(&b, &[s]);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn in_op_matches_list_member() {
        let s = scene_yaml(
            r#"
id: x
version: 1
describe: {template: x}
match:
  all:
    - { feature: context.app, op: in, value: [Zoom, Teams, Feishu] }
"#,
        );
        let mut b = FeatureBundle::new(0.0);
        b.insert(
            FeatureKey::new("context.app").unwrap(),
            FeatureValue::Category("Feishu".into()),
        );
        assert_eq!(SimpleRuleMatcher::new().match_scenes(&b, &[s]).len(), 1);
    }
}
