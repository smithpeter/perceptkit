//! Flapping-resistant scene transition FSM.
//!
//! Core of Stability dimension (STRATEGY §2). Defends against scene
//! classification noise via:
//!
//! - **Hysteresis**: `enter_hi` (0.70) > `exit_lo` (0.55)
//! - **Dwell**: candidate must persist `dwell` (3s) before committing
//! - **Hot switch**: confidence ≥ `hot_switch_floor` (0.85) bypasses dwell
//! - **Uncertain state**: confidence collapse → "thinking" (no flap UI)
//!
//! Property tested in `tests::no_flap_under_noise`: 60s signal centered
//! at 0.65 ±0.05 → ≤1 transition.

use std::time::Duration;

/// Tunable thresholds for the FSM.
#[derive(Debug, Clone)]
pub struct FsmConfig {
    /// Confidence at/above which a candidate can eventually transition.
    pub enter_hi: f64,
    /// Confidence below which the current scene is considered lost.
    pub exit_lo: f64,
    /// Minimum candidate persistence before committing transition.
    pub dwell: Duration,
    /// Confidence at/above which a candidate bypasses dwell entirely.
    pub hot_switch_floor: f64,
}

impl Default for FsmConfig {
    fn default() -> Self {
        Self {
            enter_hi: 0.70,
            exit_lo: 0.55,
            dwell: Duration::from_secs(3),
            hot_switch_floor: 0.85,
        }
    }
}

/// Output of `FlappingFsm::step`.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionOutput {
    /// Hold current scene (or None) without transition.
    Hold {
        /// Current scene id if any.
        scene: Option<String>,
    },
    /// Transition committed from `from` → `to`.
    Transition {
        /// Previous scene (None if initial).
        from: Option<String>,
        /// New scene id.
        to: String,
    },
    /// Enter "thinking" — show uncertainty rather than flap.
    Uncertain {
        /// Most recent scene (for context display).
        last: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum FsmState {
    Initial,
    Stable {
        scene: String,
    },
    Pending {
        current: Option<String>,
        proposed: String,
        proposed_since: f64,
    },
    Uncertain,
}

/// Flapping-resistant scene transition FSM.
///
/// Call `step(scene, confidence, now)` per evaluation tick. The FSM decides
/// whether to hold, transition, or enter Uncertain based on the config.
#[derive(Debug, Clone)]
pub struct FlappingFsm {
    config: FsmConfig,
    state: FsmState,
}

impl FlappingFsm {
    /// Construct with custom config.
    pub fn new(config: FsmConfig) -> Self {
        Self {
            config,
            state: FsmState::Initial,
        }
    }

    /// Default config (STRATEGY §2 values).
    pub fn default_config() -> Self {
        Self::new(FsmConfig::default())
    }

    /// Current scene id, if any.
    pub fn current_scene(&self) -> Option<&str> {
        match &self.state {
            FsmState::Stable { scene } => Some(scene),
            FsmState::Pending { current, .. } => current.as_deref(),
            _ => None,
        }
    }

    /// Whether the FSM is currently in `Uncertain` state.
    pub fn is_uncertain(&self) -> bool {
        matches!(self.state, FsmState::Uncertain)
    }

    /// Step the FSM forward with a new classifier output.
    ///
    /// - `scene`: classifier's top scene id, or `None` if classifier said Unknown.
    /// - `confidence`: classifier's top-1 confidence in `[0.0, 1.0]`.
    /// - `now`: wall-clock time in seconds (monotonic preferred).
    pub fn step(&mut self, scene: Option<&str>, confidence: f64, now: f64) -> TransitionOutput {
        let cfg = self.config.clone();

        // Hot switch: very high confidence → bypass dwell.
        if let Some(s) = scene {
            if confidence >= cfg.hot_switch_floor {
                let from = self.current_scene().map(String::from);
                let to = s.to_string();
                self.state = FsmState::Stable { scene: to.clone() };
                return if from.as_deref() == Some(s) {
                    TransitionOutput::Hold { scene: Some(to) }
                } else {
                    TransitionOutput::Transition { from, to }
                };
            }
        }

        let state = self.state.clone();
        match state {
            FsmState::Initial => self.handle_initial(scene, confidence, now),
            FsmState::Stable { scene: current } => {
                self.handle_stable(&current, scene, confidence, now)
            }
            FsmState::Pending {
                current,
                proposed,
                proposed_since,
            } => self.handle_pending(current, proposed, proposed_since, scene, confidence, now),
            FsmState::Uncertain => self.handle_uncertain(scene, confidence, now),
        }
    }

    fn handle_initial(
        &mut self,
        scene: Option<&str>,
        confidence: f64,
        _now: f64,
    ) -> TransitionOutput {
        if let Some(s) = scene {
            if confidence >= self.config.enter_hi {
                self.state = FsmState::Stable {
                    scene: s.to_string(),
                };
                return TransitionOutput::Transition {
                    from: None,
                    to: s.to_string(),
                };
            }
        }
        TransitionOutput::Hold { scene: None }
    }

    fn handle_stable(
        &mut self,
        current: &str,
        scene: Option<&str>,
        confidence: f64,
        now: f64,
    ) -> TransitionOutput {
        // Confidence collapsed below exit_lo → Uncertain.
        if confidence < self.config.exit_lo {
            self.state = FsmState::Uncertain;
            return TransitionOutput::Uncertain {
                last: Some(current.to_string()),
            };
        }

        match scene {
            Some(s) if s == current => {
                // Stable, no action needed.
                TransitionOutput::Hold {
                    scene: Some(current.to_string()),
                }
            }
            Some(s) if confidence >= self.config.enter_hi => {
                // Different scene, high conf → enter pending.
                self.state = FsmState::Pending {
                    current: Some(current.to_string()),
                    proposed: s.to_string(),
                    proposed_since: now,
                };
                TransitionOutput::Hold {
                    scene: Some(current.to_string()),
                }
            }
            _ => TransitionOutput::Hold {
                scene: Some(current.to_string()),
            },
        }
    }

    fn handle_pending(
        &mut self,
        current: Option<String>,
        proposed: String,
        proposed_since: f64,
        scene: Option<&str>,
        confidence: f64,
        now: f64,
    ) -> TransitionOutput {
        let dwell_s = self.config.dwell.as_secs_f64();

        // Input returns to current scene with good conf → abandon candidate.
        if let (Some(cur), Some(s)) = (current.as_deref(), scene) {
            if s == cur && confidence >= self.config.exit_lo {
                self.state = FsmState::Stable {
                    scene: cur.to_string(),
                };
                return TransitionOutput::Hold {
                    scene: Some(cur.to_string()),
                };
            }
        }

        match scene {
            Some(s) if s == proposed.as_str() => {
                if confidence >= self.config.enter_hi {
                    if now - proposed_since >= dwell_s {
                        // Dwell elapsed — commit transition.
                        self.state = FsmState::Stable {
                            scene: proposed.clone(),
                        };
                        TransitionOutput::Transition {
                            from: current,
                            to: proposed,
                        }
                    } else {
                        // Still waiting for dwell.
                        self.state = FsmState::Pending {
                            current: current.clone(),
                            proposed,
                            proposed_since,
                        };
                        TransitionOutput::Hold { scene: current }
                    }
                } else {
                    // Proposed dropped — revert.
                    self.revert_from_pending(current, proposed_since, now)
                }
            }
            Some(s) if confidence >= self.config.enter_hi => {
                // A third scene jumped in — restart pending with the new target.
                self.state = FsmState::Pending {
                    current: current.clone(),
                    proposed: s.to_string(),
                    proposed_since: now,
                };
                TransitionOutput::Hold { scene: current }
            }
            _ => self.revert_from_pending(current, proposed_since, now),
        }
    }

    fn revert_from_pending(
        &mut self,
        current: Option<String>,
        last_stable_since: f64,
        now: f64,
    ) -> TransitionOutput {
        let _ = last_stable_since;
        match current.clone() {
            Some(c) => {
                self.state = FsmState::Stable { scene: c.clone() };
                TransitionOutput::Hold { scene: Some(c) }
            }
            None => {
                let _ = now;
                self.state = FsmState::Uncertain;
                TransitionOutput::Uncertain { last: None }
            }
        }
    }

    fn handle_uncertain(
        &mut self,
        scene: Option<&str>,
        confidence: f64,
        _now: f64,
    ) -> TransitionOutput {
        if let Some(s) = scene {
            if confidence >= self.config.enter_hi {
                self.state = FsmState::Stable {
                    scene: s.to_string(),
                };
                return TransitionOutput::Transition {
                    from: None,
                    to: s.to_string(),
                };
            }
        }
        TransitionOutput::Hold { scene: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn fsm() -> FlappingFsm {
        FlappingFsm::default_config()
    }

    #[test]
    fn initial_with_high_conf_transitions() {
        let mut f = fsm();
        let out = f.step(Some("office"), 0.80, 0.0);
        assert!(matches!(
            out,
            TransitionOutput::Transition { from: None, to }
            if to == "office"
        ));
    }

    #[test]
    fn initial_with_low_conf_holds() {
        let mut f = fsm();
        let out = f.step(Some("office"), 0.60, 0.0);
        assert_eq!(out, TransitionOutput::Hold { scene: None });
    }

    #[test]
    fn stable_ignores_mild_noise() {
        let mut f = fsm();
        f.step(Some("office"), 0.80, 0.0);
        // Dip to 0.65 — above exit_lo, below enter_hi → hold.
        let out = f.step(Some("office"), 0.65, 0.5);
        assert_eq!(
            out,
            TransitionOutput::Hold {
                scene: Some("office".into())
            }
        );
    }

    #[test]
    fn stable_enters_uncertain_on_collapse() {
        let mut f = fsm();
        f.step(Some("office"), 0.80, 0.0);
        let out = f.step(Some("office"), 0.30, 1.0);
        assert!(matches!(out, TransitionOutput::Uncertain { .. }));
        assert!(f.is_uncertain());
    }

    #[test]
    fn stable_respects_dwell_before_transition() {
        let mut f = fsm();
        f.step(Some("office"), 0.80, 0.0);
        // Seeing "meeting" at 0.75 (above enter_hi, below hot_switch).
        let out = f.step(Some("meeting"), 0.75, 1.0);
        // Should hold, enter Pending.
        assert!(matches!(out, TransitionOutput::Hold { .. }));
        // Two more ticks within dwell → still hold.
        assert!(matches!(
            f.step(Some("meeting"), 0.75, 2.0),
            TransitionOutput::Hold { .. }
        ));
        // After 3+ seconds since first "meeting" hit → commit.
        let out = f.step(Some("meeting"), 0.75, 4.5);
        assert!(matches!(
            out,
            TransitionOutput::Transition { from: Some(_), to } if to == "meeting"
        ));
    }

    #[test]
    fn hot_switch_bypasses_dwell() {
        let mut f = fsm();
        f.step(Some("office"), 0.80, 0.0);
        let out = f.step(Some("driving"), 0.90, 0.2);
        assert!(matches!(
            out,
            TransitionOutput::Transition { from: Some(_), to } if to == "driving"
        ));
    }

    #[test]
    fn returning_to_current_cancels_pending() {
        let mut f = fsm();
        f.step(Some("office"), 0.80, 0.0);
        // Pending → meeting
        f.step(Some("meeting"), 0.75, 1.0);
        // Back to office → cancel pending
        let out = f.step(Some("office"), 0.75, 2.0);
        assert_eq!(
            out,
            TransitionOutput::Hold {
                scene: Some("office".into())
            }
        );
        // Now further meeting hits must wait full dwell again
        f.step(Some("meeting"), 0.75, 3.0);
        assert!(matches!(
            f.step(Some("meeting"), 0.75, 4.0),
            TransitionOutput::Hold { .. }
        ));
    }

    #[test]
    fn proposed_drops_conf_reverts_to_current() {
        let mut f = fsm();
        f.step(Some("office"), 0.80, 0.0);
        f.step(Some("meeting"), 0.75, 1.0);
        // meeting confidence drops
        let out = f.step(Some("meeting"), 0.40, 2.0);
        assert_eq!(
            out,
            TransitionOutput::Hold {
                scene: Some("office".into())
            }
        );
    }

    // --- Property-based: no flap under noisy signals near 0.65 ---
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn no_flap_under_mild_noise(
            seed in 0u64..1000,
        ) {
            // Simulate 60s, 10 Hz ticks (600 samples), confidence N(0.65, 0.05)
            // clamped to [0.50, 0.80]. Scene alternates A/B with 30% bias to A.
            let mut rng_state = seed;
            let rand = |s: &mut u64| {
                *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (*s >> 32) as u32 as f64 / u32::MAX as f64
            };
            let mut f = fsm();
            f.step(Some("A"), 0.80, 0.0); // Start settled at A
            let mut transitions = 0;
            for i in 0..600 {
                let t = 0.1 * (i as f64) + 0.1;
                let base = 0.65;
                let noise = (rand(&mut rng_state) - 0.5) * 0.10;
                let conf = (base + noise).clamp(0.0, 1.0);
                let scene = if rand(&mut rng_state) < 0.3 { "B" } else { "A" };
                let out = f.step(Some(scene), conf, t);
                if let TransitionOutput::Transition { .. } = out {
                    transitions += 1;
                }
            }
            // Staying near 0.65 (below enter_hi=0.70) must never cross the
            // threshold → expected 0 transitions. Allow ≤1 as safety margin.
            prop_assert!(transitions <= 1, "expected ≤1 transition, got {transitions}");
        }
    }
}
