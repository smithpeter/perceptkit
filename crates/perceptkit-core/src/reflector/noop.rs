//! NoopReflector — always returns `Unknown`. The CI default.
//!
//! Used when:
//! - Core compiled without any LLM feature
//! - Offline / edge deployments
//! - Tests that verify hot path in isolation

use async_trait::async_trait;

use super::{PendingCase, PromptHash, ReflectError, Reflection, Reflector};

/// Reflector that always returns `Reflection::Unknown`.
#[derive(Debug, Default, Clone)]
pub struct NoopReflector;

impl NoopReflector {
    /// Create an instance.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Reflector for NoopReflector {
    async fn reflect(&self, _case: PendingCase) -> Result<Reflection, ReflectError> {
        Ok(Reflection::unknown(
            "NoopReflector: no LLM available. Enable feature `local-reflector` or use `MockReflector` in tests.",
        ))
    }

    fn name(&self) -> &'static str {
        "noop"
    }

    fn fingerprint(&self) -> PromptHash {
        PromptHash("noop@v1".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::SceneDecision;

    #[tokio::test]
    async fn noop_returns_unknown() {
        let r = NoopReflector::new();
        let case = PendingCase {
            id: "test".into(),
            timestamp: 0.0,
            features: vec![],
            reason: "test".into(),
            failed_decision: SceneDecision::unknown(),
        };
        let refl = r.reflect(case).await.unwrap();
        matches!(refl, Reflection::Unknown { .. });
    }

    #[tokio::test]
    async fn noop_has_stable_fingerprint() {
        let r = NoopReflector::new();
        assert_eq!(r.fingerprint(), PromptHash("noop@v1".into()));
        assert_eq!(r.name(), "noop");
    }
}
