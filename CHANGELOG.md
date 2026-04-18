# Changelog

All notable changes to perceptkit will be documented in this file.

Follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [0.1.0-alpha.0] — 2026-04-18

### Added

**M1 — Scaffold**
- Cargo workspace with 4 crates: `perceptkit-core` / `perceptkit-audio` / `perceptkit-py` / `perceptkit-cli`
- PyO3 abi3-py311 bindings via maturin
- Dual licensing (MIT OR Apache-2.0)
- `cargo deny` configuration enforcing Signal Model (no network crates)
- GitHub Actions CI (Rust fmt / clippy / test / cargo-deny / DCO / offline-build / eval-gate)
- DCO git hook + CI enforcement

**M2 — Core + Dual-Process**
- Signal / FeatureDescriptor (typed) / FeatureBundle / FeatureKey (validated newtype)
- Scene YAML DSL with all/any/none conditions, 8 operators, priority
- FeatureRegistry with **Levenshtein `did_you_mean`** typo detection
- SimpleRuleMatcher + PriorityArbiter + ThresholdGate
- Reflector trait (async) + NoopReflector
- PendingSceneQueue (SQLite) for human-in-the-loop scene discovery
- SceneEngine.from_dir / evaluate / evaluate_async / lint

**M3 — Audio + Flapping FSM**
- 4-state FSM: Initial / Stable / Pending / Uncertain
- Hysteresis (enter_hi=0.70 / exit_lo=0.55) + dwell (3s) + hot_switch (0.85)
- proptest: 60s signal at 0.65±0.05 → ≤1 transition
- EnergyExtractor (RMS / peak / dBFS)
- VoiceActivityExtractor (energy + ZCR)
- MultiSpeakerExtractor (stub, v0.2 CAM++ integration)
- AudioProvider orchestrator

**M4 — Evaluation**
- `perceptkit synthesize` — deterministic JSONL dataset generator
- `perceptkit eval --gate` — Top-1 / Macro-F1 / per-scene recall with v0.1 thresholds
- CI evaluation-gate job on synthetic fixture (100% pass)

**M5 — Python API**
- `SceneEngine.from_dir(path)` / `analyze_audio(np.ndarray, sr)` / `analyze_bundle(dict)`
- `SceneDecision` class with scene_id / confidence / description / source / rationale
- `SceneEngine.lint(path)` → dict
- 12 python tests (end-to-end silent audio → office_quiet, meeting features → online_meeting)

**M6 — Reflector + Evolution**
- MockReflector (VCR fixture pattern)
- `perceptkit review list/approve/reject` — human review of LLM proposals
- **approve writes YAML to scenes/** — completes Evolution Loop (never auto-commits)
- `perceptkit reflect` — single-invocation Reflector trigger

**M7 — Release Prep**
- `.github/workflows/release.yml` — manual-triggered wheel build + optional crates.io/PyPI publish (dry_run default)
- `docs/voxsign-integration.md` — integration contract + rollout plan
- `docs/blog-1-benchmark.md` — HN release draft

### Governance
- 3 rounds of red/blue team adversarial review on strategy (weighted 7.25/10 GO)
- 1 round on data strategy (4 perspectives: architecture+privacy / product+community / business+investment / QA+datascience)
- 5 user strategic decisions: name=perceptkit, v0.1=B (Dual-Process), positioning=B (Agent Context Layer), commercial=D (VoxSign moat + personal brand), LLM-threat=A+B+C+D combined
- 3 data micro-decisions: scope=A2+A3 (v0.2 community, v0.3+ flywheel deferred), license=B2 (CC-BY-NC main + CC0 contrib), repo=C2, VoxSign data=D2+D1 (never public), docs=E3

### Known Limitations (v0.1 → v0.2 roadmap)
- Real 525-clip human-labeled benchmark (needs $400 Prolific budget + 2 labelers + kappa ≥0.70)
- Qwen-0.5B LocalReflector (needs llama-cpp-2 + model weights)
- Temporal DSL ("past 30s"), Stateful DSL, Per-user overrides
- Vision / Text / Context provider modalities
