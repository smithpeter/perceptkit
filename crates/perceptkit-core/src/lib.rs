//! perceptkit-core — Core traits and DSL for perceptkit.
//!
//! perceptkit is the **perception middleware for AI agents**: it turns
//! multimodal signals (audio, context, vision-later) into declarative,
//! auditable scene decisions.
//!
//! # Architecture
//!
//! Dual-Process (Kahneman-style):
//!
//! - **Hot Path**: Signal → Feature → RuleMatcher → Arbiter → high-confidence decision
//! - **Cold Path**: ConfidenceGate escalates to Reflector (LLM tool-calling agent)
//! - **Evolution Loop**: LLM-proposed scenes → PendingQueue → human review → scenes/*.yaml
//!
//! See `STRATEGY.md §3` for the full architecture.
//!
//! # Signal Model Commitment
//!
//! `perceptkit-core` **must not** depend on any network crate. This is enforced
//! by `cargo deny` in CI. See `DATA.md §2`.
//!
//! # v0.1 Scaffold
//!
//! This is the M1 scaffold. The real traits land in M2:
//! - `Signal` / `Modality` — signal bus
//! - `FeatureDescriptor` / `FeatureBundle` — typed feature system
//! - `Scene` / `SceneEngine` / `Arbiter` / `ConfidenceGate` — hot path
//! - `Reflector` / `Reflection` — cold path
//! - `PendingSceneQueue` — evolution loop

#![forbid(unsafe_code)]

/// Version of `perceptkit-core`, for telemetry-free audit trails.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.starts_with("0."));
    }
}
