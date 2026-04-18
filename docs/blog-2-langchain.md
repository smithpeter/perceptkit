# Draft: "Why Your LangChain Agent Is Deaf — and how perceptkit fixes it"

> Target: dev.to / Medium / HN follow-up to blog-1
> Length: 1200-1500 words
> Status: draft, polish for M7 release

---

## Lede

Open any LangChain tutorial in 2026. You'll see prompts, tool-calling,
retrieval, memory. What you won't see: any concept of **where the user is
right now**.

Your agent doesn't know if you're in a meeting. It doesn't know if you're
driving. It doesn't know if you're alone in a quiet office or in a noisy
café with a friend.

That's not a bug in LangChain. LangChain is an orchestration framework.
"Environment sensing" is a different layer — one nobody has built.

Today I'm releasing **perceptkit**, the layer LangChain forgot.

---

## The LangChain stack, honestly

Here's what a typical LangChain agent knows:

```
┌─────────────────────────────┐
│ User prompt (text)          │
├─────────────────────────────┤
│ Chat history                │
├─────────────────────────────┤
│ Retrieved documents         │
├─────────────────────────────┤
│ Available tools             │
└─────────────────────────────┘
```

Here's what it doesn't know:

```
┌─────────────────────────────┐
│ Is the user speaking?       │
│ Is there background noise?  │
│ Is the user alone?          │
│ Which app is active?        │
│ Is the user driving?        │
│ Is it late at night?        │
└─────────────────────────────┘
```

Every one of these is a **scene**. Scenes change. Agents that don't adapt
to scenes are pathologically rude.

---

## Why is this hole empty?

Three reasons.

**(1) Scene detection is boring ML engineering.** No one gets a PhD for
"energy threshold + zero-crossing rate + SNR". So the academic community
moved on to transformers. Meanwhile Silero VAD solved the basic voice
detection problem *in C++, for free*, 5 years ago. The industry never
upgraded the abstraction.

**(2) Each product builds its own 1741 lines of scene rules.** I've seen
this three times now. VoxSign has 1741 lines of hand-written scene
classification code. Wispr Flow probably has similar. Granola too. No one
shares because no one thought to extract the abstraction.

**(3) LLMs *seem* like they could solve it.** "Just ask GPT-4o what scene
it is." And they can. But at $0.03/call and 2s latency, you're not asking
for every 100ms audio frame. You're asking once every 30 seconds. And
that's when the user walks out of the meeting and the agent wakes them up
at home at 2am because its scene cache was stale.

---

## perceptkit's model: Dual-Process

Kahneman says humans have System 1 (fast, automatic) and System 2 (slow,
reasoning). Your agent's perception layer should too.

**Hot Path (System 1)**: deterministic, explicit, **1.77 µs on my laptop**.
Scenes defined in YAML. Rule engine with priority-based arbitration. A
4-state flapping FSM so your scene doesn't oscillate when the user moves
the microphone. No network, ever.

**Cold Path (System 2)**: when the rule engine is uncertain, escalate to
an LLM (Qwen-0.5B locally, GPT-5-mini in the cloud, your choice). The
LLM's job is bounded: Map this case to a known scene, Propose a new one,
or honestly say Unknown. Every Cold Path call is logged as a
ReflectionTrace you can replay.

**Evolution Loop**: when the LLM proposes a new scene, it enters a human
review queue. A maintainer (you, presumably) runs `perceptkit review
approve` and the proposed YAML lands in your scenes/ directory. No
auto-commits. Ever. That's a deliberate design choice — STRATEGY §11.3.

---

## What you actually write

A scene is YAML:

```yaml
id: driving_with_passenger
version: 1
describe:
  template: "Driving with someone in the car"
match:
  all:
    - { feature: context.motion, op: eq, value: vehicle }
    - { feature: audio.voice_ratio, op: gt, value: 0.5 }
    - { feature: audio.speaker_count, op: gte, value: 2 }
priority: 20
```

The engine wires this into a chain that evaluates in 1.77 µs. Add new
scenes without recompiling anything. Type a feature name wrong and the
loader tells you "did you mean `audio.voice_ratio`?"

In Python:

```python
import perceptkit as pk
engine = pk.SceneEngine.from_dir("./scenes")
decision = engine.analyze_audio(pcm_np, 16000)
if decision.scene_id == "driving_with_passenger":
    agent.mute_tts()  # don't distract the driver
```

That's it. No training, no fine-tuning, no embedding store.

---

## The honest trade-offs

perceptkit is not magic. In v0.1:

- Accuracy is 78% Top-1 on synthetic data. Real 525-clip benchmark is in
  progress; the goalpost is 85% for v0.2.
- We don't support "past 30 seconds" temporal rules or "if the previous
  scene was X" stateful logic. v0.2.
- LocalReflector (Qwen) is scaffolded but requires you to wire llama-cpp-2
  yourself. The MockReflector is production-ready for tests.
- Moat is weak; if LangChain merges `langchain_community.perception` next
  week, the value goes from "novel library" to "helpful Rust backend".
  Fine by me — the point is that **your agent gets scene awareness**, not
  that I own the category.

These honest limitations are listed in the README. We'd rather you know
up front than discover after you've built 3 months of product on top.

---

## Why Rust

Because the car's head unit doesn't run Python. Neither does the
microcontroller in the smart lock. If perception lives only where Python
runs, it doesn't live where your agent needs to be.

Rust core + PyO3 binding + (future) iOS/Swift binding + (future) WASM =
same scene library, everywhere.

---

## Call to action

Go read the code. It's 7000 lines of Rust. 87 tests. Full doc suite.

Go write a scene. If your agent does *anything* adaptive — don't interrupt
the user, wake them up, mute TTS — you need scene awareness. perceptkit
is the fastest path.

Go push a PR. DCO (signed-off-by), not CLA. We want your `shopping`
scene, your `dentist_waiting_room` scene, whatever your product actually
cares about.

Repo: `github.com/smithpeter/perceptkit`

---

*This is infrastructure work. It won't get cited. It might get forked
into LangChain and the name might die. That's fine. What matters is that
your agent knows whether you're driving.*
