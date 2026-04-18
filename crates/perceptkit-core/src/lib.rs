//! perceptkit-core — Core traits and DSL for perceptkit.
//!
//! See `STRATEGY.md` for the North Star. `DATA.md §2` for the Signal Model
//! (no network dependencies in core, enforced by `cargo deny`).
//!
//! # Dual-Process Architecture
//!
//! - Hot Path: `FeatureBundle` → `RuleMatcher` → `Arbiter` → `SceneDecision`
//! - Confidence Gate: low-conf/ambiguous → escalate
//! - Cold Path: `Reflector` (LLM tool-calling agent) → `Reflection` (Map / Propose / Unknown)
//! - Evolution Loop: proposed scenes → `PendingSceneQueue` → human review → `scenes/*.yaml`

#![forbid(unsafe_code)]

pub mod dsl;
pub mod engine;
pub mod error;
pub mod feature;
pub mod gate;
pub mod matcher;
pub mod queue;
pub mod reflector;
pub mod registry;
pub mod scene;
pub mod signal;
pub mod transition;

pub use engine::{LintReport, SceneEngine};
pub use error::{Error, Result};
pub use feature::{
    FeatureBundle, FeatureDescriptor, FeatureKey, FeatureKind, FeatureValue, TimeWindow,
};
pub use gate::{ConfidenceGate, GateVerdict, ThresholdGate};
pub use matcher::{Arbiter, EvalCtx, PriorityArbiter, RuleMatcher, SceneMatch, SimpleRuleMatcher};
pub use queue::{PendingRow, PendingSceneQueue, PendingStatus};
pub use reflector::{
    MockReflector, NoopReflector, PendingCase, Reflection, ReflectionBudget, Reflector,
};
pub use registry::FeatureRegistry;
pub use scene::{DecisionSource, Evidence, EvidenceKind, Scene, SceneDecision};
pub use signal::{Modality, Signal};
pub use transition::{FlappingFsm, FsmConfig, TransitionOutput};

/// Version of `perceptkit-core`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
