//! MockReflector — deterministic pre-programmed responses for testing.
//!
//! Each call to `reflect()` pops the next queued `Reflection`. When the
//! queue empties, subsequent calls return `ReflectError::Backend("exhausted")`.
//! This is the VCR-style fixture replay pattern for reproducible LLM tests.

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use super::{PendingCase, PromptHash, ReflectError, Reflection, Reflector};

/// Pre-programmed reflector — pops queued responses in order.
pub struct MockReflector {
    responses: Mutex<VecDeque<Reflection>>,
    fingerprint: PromptHash,
}

impl MockReflector {
    /// Build from a list of pre-programmed reflections.
    pub fn new(responses: Vec<Reflection>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            fingerprint: PromptHash("mock@v1".into()),
        }
    }

    /// Remaining queued responses.
    pub fn remaining(&self) -> usize {
        self.responses.lock().map(|q| q.len()).unwrap_or(0)
    }
}

#[async_trait]
impl Reflector for MockReflector {
    async fn reflect(&self, _case: PendingCase) -> Result<Reflection, ReflectError> {
        let mut q = self
            .responses
            .lock()
            .map_err(|_| ReflectError::Backend("mock reflector mutex poisoned".into()))?;
        q.pop_front()
            .ok_or_else(|| ReflectError::Backend("mock reflector exhausted".into()))
    }

    fn name(&self) -> &'static str {
        "mock"
    }

    fn fingerprint(&self) -> PromptHash {
        self.fingerprint.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::SceneDecision;

    fn case() -> PendingCase {
        PendingCase {
            id: "c1".into(),
            timestamp: 0.0,
            features: vec![],
            reason: "test".into(),
            failed_decision: SceneDecision::unknown(),
        }
    }

    #[tokio::test]
    async fn pops_in_order() {
        let m = MockReflector::new(vec![
            Reflection::Map {
                scene_id: "a".into(),
                rationale: "r1".into(),
            },
            Reflection::unknown("s"),
        ]);
        assert_eq!(m.remaining(), 2);
        let out = m.reflect(case()).await.unwrap();
        matches!(out, Reflection::Map { .. });
        assert_eq!(m.remaining(), 1);
    }

    #[tokio::test]
    async fn exhausted_errors() {
        let m = MockReflector::new(vec![Reflection::unknown("s")]);
        let _ = m.reflect(case()).await.unwrap();
        let err = m.reflect(case()).await.unwrap_err();
        matches!(err, ReflectError::Backend(_));
    }
}
