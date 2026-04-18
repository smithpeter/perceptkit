# VoxSign Integration — perceptkit POC

> Status: **skeleton** (M7 in progress)
>
> Goal: replace VoxSign's `src/voxsign/edge/scene/` (2809 lines incl. 1741-line
> hard-coded taxonomy) with a perceptkit-backed `PerceptkitSceneAdapter`.

---

## 1. Scope (per STRATEGY §11.5, Round 3 decision D2+D1)

**In scope for v0.1 POC**:
- Architectural swap: replace `SceneAnalyzer` trait with `PerceptkitSceneAdapter`
- Translate **5-10 core scenes** to perceptkit YAML (not the full 1741 lines)
- Quantify coverage: what % of VoxSign's current scene logic can be expressed in v0.1 DSL

**Out of scope for v0.1**:
- VoxSign real user data → perceptkit public pipeline (✘ D2: never)
- Full 1741-line taxonomy migration (→ v0.2 with temporal/stateful DSL)
- LLM Reflector with user data (✘ circular pollution risk)

---

## 2. Architectural Approach

```
Before:
  Audio ──▶ SceneAnalyzer::analyze(audio, sr) ──▶ SceneInfo { scene_id, confidence }
           (hard-coded taxonomy + classifier)

After:
  Audio ──▶ PerceptkitSceneAdapter ──▶ SceneInfo { scene_id, confidence }
                  │
                  ▼
           perceptkit::SceneEngine
           (YAML scenes + rule engine + Flapping FSM)
```

The adapter:
- Holds a `SceneEngine` loaded from `scenes/voxsign/*.yaml`
- Converts VoxSign `AudioFrame` → perceptkit `FeatureBundle` via `AudioProvider`
- Maps perceptkit `SceneDecision` → VoxSign `SceneInfo` with confidence mapping

---

## 3. Scene Translation Coverage Audit (v0.1)

Run the audit script:

```bash
python scripts/audit_voxsign_coverage.py \
    --voxsign-src ~/VoxSign/src/voxsign/edge/scene/ \
    --perceptkit-scenes scenes/voxsign/
```

Report format:
```
VoxSign scenes (source of truth):
  - office_quiet            [translated ✓]
  - online_meeting          [translated ✓]
  - driving                 [translated ✓]
  - outdoor_noisy           [translated ✓]
  - multi_speaker_chat      [translated ✓]
  - call_phone              [temporal DSL required → v0.2]
  - podcast_recording       [stateful DSL required → v0.2]
  - ...

Coverage: 5 / N scenes (XX%)
```

**Gate**: ≥40% coverage for public v0.1 release (STRATEGY §11.5).
Below 40% → silent release, wait for v0.2 DSL expansion.

---

## 4. Data Boundary

**VoxSign real user audio is NEVER copied into perceptkit-bench-v0.**

- Evaluation of adapter uses VoxSign's internal test fixtures only
- No CEO/family/private audio contributes to perceptkit public benchmarks
- Performance numbers in this doc use only public synthetic + AudioSet subset data

This enforces STRATEGY §11.5 ("身份认证场景一次泄露 = 职业生涯终结").

---

## 5. Rollout Plan

1. **Stage 1 (current)**: POC adapter in VoxSign branch `perceptkit-integration`, not merged
2. **Stage 2 (v0.1 release)**: perceptkit-bench-v0 published; VoxSign main still uses old taxonomy
3. **Stage 3 (v0.2)**: Temporal/Stateful DSL in perceptkit; coverage audit re-runs; target ≥80%
4. **Stage 4 (VoxSign v2.11+)**: Switch default to `PerceptkitSceneAdapter`; keep old taxonomy as fallback
5. **Stage 5 (VoxSign v2.12+)**: Retire old taxonomy

---

## 6. Verification (once POC exists)

```bash
# In VoxSign repo
cd ~/VoxSign
git checkout -b perceptkit-integration
# Adapter implementation + tests
uv run pytest tests/integration/test_perceptkit_adapter.py -v
# Comparison against legacy analyzer on internal fixtures
uv run python scripts/compare_analyzers.py --fixture tests/fixtures/scenes/
```

Expected: legacy vs perceptkit agree ≥95% on translated scenes, with
transparent failures on un-translatable scenes (temporal/stateful).

---

## 7. Known Limitations (v0.1 — documented for honesty)

| Feature                  | VoxSign has it | perceptkit v0.1 DSL | Plan |
|--------------------------|----------------|---------------------|------|
| Single-tick rule match   | ✓              | ✓                   | —    |
| Temporal ("past 30s")    | ✓              | ✘                   | v0.2 |
| Stateful ("if previous") | ✓              | ✘                   | v0.2 |
| Per-user overrides       | ✓              | ✘                   | v0.2 |
| App/window context       | ✓              | ✓ (via `context.app`) | —  |
| Multi-speaker count      | ✓ (CAM++)      | stub (always 1)     | v0.2 |
| Sound event (YAMNet)     | ✓              | trait only          | v0.2 |

---

*This file exists as the contract between perceptkit and VoxSign. Updates require both
maintainers' sign-off.*
