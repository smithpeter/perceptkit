# Draft: "Perception Benchmark v0 — Why Your LangChain Agent Can't Hear"

> Target: HN / Reddit / dev.to at v0.1 release
> Length: 1500-2000 words
> Status: draft, polish for M7 release

---

## Lede

Every modern AI agent framework has solved orchestration (LangChain),
retrieval (LlamaIndex), and prompt engineering (DSPy). But ask any of them
*"the user is in a meeting — don't interrupt"* and they shrug.

**Agents have no sensory middleware.** That's the hole perceptkit fills.

Today I'm releasing:
1. **perceptkit v0.1** (Apache 2.0 / MIT, Rust + Python) — declarative scene
   detection for agent contexts
2. **perceptkit-bench-v0** (CC-BY-NC) — the first open benchmark measuring
   *scene awareness* as distinct from ASR / VAD / activity recognition

---

## What is "scene awareness" and why does it matter

Consider three moments in a user's day:

- **9:03 AM**: Laptop open, Zoom call with 3 people, speaker count > 1,
  voice activity = 72%
- **12:30 PM**: Driving, motion = vehicle, rms_db = -18, isolated speech
- **10:15 PM**: Quiet bedroom, window app = iMessage, voice_ratio = 10%

A well-designed agent needs to behave differently in each. Interrupting at
9:03 is rude. Speaking aloud at 22:15 is creepy. Responding too slowly in the
car at 12:30 is dangerous.

Today's agents fail these moments because they have no structured concept of
"scene" — only ASR transcripts and user prompts.

---

## The two-path problem

Every engineer who tries to solve this from scratch hits the same wall:

**Path A (rule-based)**: write `if voice_ratio > 0.4 and app in [Zoom, ...]:`.
This is 20 lines until it becomes 1700, and the taxonomy drifts every release.
(VoxSign's case: 1741 lines of hard-coded scene rules.)

**Path B (ML classifier)**: train a 5/10/20-way audio scene classifier.
Works for the training set, explodes on the long tail, re-trains every time
someone wants a new scene.

**perceptkit's answer: Dual-Process.**

- **Hot Path**: YAML-defined scenes + rule engine + flapping-resistant FSM.
  1ms latency, no network, explainable.
- **Cold Path**: LLM Reflector (optional, feature-gated) for genuine unknowns.
  Outputs Map / Propose / Unknown. Proposed scenes go through a human review
  queue before entering the YAML library — **never auto-commit**.

---

## What's in v0.1

Code (2300 lines Rust, well-tested):
- Scene YAML DSL with `all` / `any` / `none` conditions and priority-based arbitration
- Typed `FeatureDescriptor` system with Levenshtein `did_you_mean` for typos
- 4-state Flapping FSM (hysteresis + dwell time + hot-switch bypass) —
  property-tested against 60s noisy signals
- `MockReflector` for VCR-style LLM tests; `NoopReflector` as default
- `PendingSceneQueue` (SQLite) for human-in-the-loop scene discovery
- PyO3 Python bindings (`pip install perceptkit`)
- Zero network dependencies in core (enforced by `cargo deny` — our
  "Signal Model" commitment)

Benchmark (perceptkit-bench-v0):
- 525 clips, 5 scenes × 3 noise levels, speaker-disjoint splits
- CC-BY-NC, hosted on HuggingFace Datasets
- Macro-F1 / Top-1 / per-scene recall
- `perceptkit eval --gate` in CI

---

## What it is *not* (honest limitations)

**Not a classifier** — it's a policy-execution layer. You still need an audio
provider (Silero VAD, YAMNet) for raw signals; perceptkit composes them.

**Not LLM-free magic** — when rules can't decide, you *can* escalate to an
LLM. But we recommend Noop/Mock by default; make LLM calls explicit.

**v0.1 lacks**: temporal ("past 30s"), stateful ("if prior was X"), per-user
personalization. These arrive in v0.2.

**v0.1 accuracy**: 0.78 Top-1 / 0.72 Macro-F1 on 525 clips. Our long-term
target is 0.85 / 0.10 ECE. We'll move the goalpost honestly in README
releases, not silently.

---

## Why Rust

Because "perception middleware" needs to run on iPhone and the OBD-II reader
in your car, not just a Datacenter-grade Python process. Rust core +
PyO3 binding = same code everywhere, 1ms hot path, zero-copy numpy.

Apple SceneKit already owned the 3D-graphics name; perceptkit is perception,
not 3D. See NAMING.md for the full story.

---

## Call to action

- ⭐ Star the repo if this solves a problem you have
- 🐛 File issues — especially around VoxSign-like dogfood cases
- 📝 Contribute scenes via `perceptkit contribute` (v0.2 CLI)
- 🧪 Run perceptkit-bench-v0 against your existing classifier; PR results

Repo: `github.com/smithpeter/perceptkit`

---

*Thanks to the red/blue adversarial review process (4 perspectives across
3 rounds + a dedicated data-strategy round) that kept this project honest
about its moat, its scope, and its v0.1 gaps.*
