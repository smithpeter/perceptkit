//! Scene YAML schema — serde-parseable types matching the DSL grammar.
//!
//! Example YAML:
//! ```yaml
//! id: online_meeting
//! version: 1
//! describe:
//!   template: "{participants}人在线会议"
//! match:
//!   all:
//!     - { feature: audio.voice_ratio, op: gt, value: 0.4 }
//!   any:
//!     - { feature: context.app, op: in, value: [zoom.us, Teams, Feishu] }
//! priority: 10
//! ```

use serde::{Deserialize, Serialize};

/// Description section with template and optional field fillers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Describe {
    /// Template string, `{field}` placeholders.
    pub template: String,
    /// Optional field mapping (placeholder → feature key / default).
    #[serde(default)]
    pub fields: std::collections::HashMap<String, Field>,
}

/// Field filler for describe template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Feature key to read value from.
    pub from: String,
    /// Default text when feature absent.
    #[serde(default)]
    pub default: Option<String>,
}

/// Match rules — `all` + `any` conditions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchRules {
    /// All of these conditions must hold.
    #[serde(default)]
    pub all: Vec<Condition>,
    /// At least one of these conditions must hold.
    #[serde(default)]
    pub any: Vec<Condition>,
    /// None of these must hold.
    #[serde(default)]
    pub none: Vec<Condition>,
}

impl MatchRules {
    /// Whether the rule set is fully empty (vacuous truth).
    pub fn is_empty(&self) -> bool {
        self.all.is_empty() && self.any.is_empty() && self.none.is_empty()
    }

    /// Iterate over all conditions for feature-reference validation.
    pub fn all_conditions(&self) -> impl Iterator<Item = &Condition> {
        self.all.iter().chain(&self.any).chain(&self.none)
    }
}

/// Single match condition — feature `op` value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Feature key to inspect.
    pub feature: String,
    /// Comparison operator.
    pub op: Op,
    /// Value to compare against.
    pub value: Value,
}

/// Supported comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// Greater than (`>`).
    Gt,
    /// Greater than or equal (`>=`).
    Gte,
    /// Less than (`<`).
    Lt,
    /// Less than or equal (`<=`).
    Lte,
    /// Equal (`==`).
    Eq,
    /// Not equal (`!=`).
    Ne,
    /// Feature value is in a list of options.
    In,
    /// Feature value is NOT in a list.
    NotIn,
}

/// YAML-parseable value (scalar or list).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Numeric value.
    Number(f64),
    /// Boolean value.
    Bool(bool),
    /// String (or category label).
    String(String),
    /// List of values (for `in` / `not_in`).
    List(Vec<Value>),
}

impl Value {
    /// Numeric extraction helper.
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Boolean extraction helper.
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// String extraction helper.
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// List extraction helper.
    pub fn as_list(&self) -> Option<&[Value]> {
        if let Self::List(l) = self {
            Some(l)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_scene_yaml() {
        let yaml = r#"
id: online_meeting
version: 1
describe:
  template: "online meeting"
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.4 }
  any:
    - { feature: context.app, op: in, value: [zoom.us, Teams] }
priority: 10
"#;
        let scene: crate::scene::Scene = serde_yml::from_str(yaml).unwrap();
        assert_eq!(scene.id, "online_meeting");
        assert_eq!(scene.priority, 10);
        assert_eq!(scene.match_rules.all.len(), 1);
        assert_eq!(scene.match_rules.all[0].op, Op::Gt);
        assert_eq!(scene.match_rules.any[0].op, Op::In);
    }

    #[test]
    fn value_helpers() {
        assert_eq!(Value::Number(1.5).as_f64(), Some(1.5));
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::String("x".into()).as_str(), Some("x"));
        assert_eq!(Value::List(vec![]).as_list().map(|l| l.len()), Some(0));
    }
}
