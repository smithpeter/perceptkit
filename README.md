# perceptkit

> **The Perception Middleware for AI agents** — declarative scenes, offline-first, self-learning.

[![CI](https://github.com/smithpeter/perceptkit/actions/workflows/ci.yml/badge.svg)](https://github.com/smithpeter/perceptkit/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Status: v0.1.0-alpha](https://img.shields.io/badge/status-v0.1.0--alpha-orange.svg)](CHANGELOG.md)

perceptkit turns multimodal signals (audio, context, vision-later) into **auditable scene decisions**, so AI agents know when to speak, when to listen, and when to escalate to an LLM.

It fills the "perception" gap in the LangChain / LlamaIndex / Haystack ecosystem.

## Status

🚧 **v0.1 in development** — M1 scaffold (2026-04-18). Not yet functional. See [plan.md](plan.md) for the 66-day roadmap.

## Why perceptkit

- **Declarative**: Scenes defined in YAML, not hard-coded classifiers.
- **Offline-first**: `perceptkit-core` has zero network dependencies (see [Signal Model](DATA.md#2-signal-model-承诺最硬核)).
- **Self-learning**: LLM Reflector proposes new scenes → human reviews → YAML library grows.
- **Fast**: Rust core, hot path p95 < 5ms, zero-copy numpy bindings.

## Architecture

```
Signal → Feature (typed) → Hot Path (rules) ──┐
                                              ├─→ Confidence Gate
                                              │
              Cold Path (Reflector: Noop/Mock/Qwen-0.5B) ←┘
                   ↓
              Scene Decision (with rationale + evidence)
                   ↓
         Evolution Loop (PendingQueue → human review → scenes/*.yaml)
```

See [STRATEGY.md](STRATEGY.md) for the North Star, [plan.md](plan.md) for the roadmap.

## Quickstart (M4+, not yet available)

```bash
# Rust
cargo add perceptkit-core perceptkit-audio

# Python
pip install perceptkit
```

```python
import numpy as np
import perceptkit as pk

engine = pk.SceneEngine.from_dir("./scenes")
decision = engine.analyze_audio(pcm_np, sample_rate=16000)
print(decision.scene_id, decision.confidence, decision.rationale)
```

## 中文

**perceptkit 是 AI Agent 的感知中间件**——把多模态信号（音频 / 视觉 / 上下文 / 文本）转成声明式、可审计、可离线、可自学习的场景决策。填补 LangChain / LlamaIndex 生态里缺失的"感知"那一格。

当前处于 v0.1 开发阶段（M1 脚手架完成）。详见：
- [STRATEGY.md](STRATEGY.md) — 项目北极星战略
- [plan.md](plan.md) — 66 工作日实施路线图
- [DATA.md](DATA.md) — 数据治理与 Signal 模型承诺
- [NAMING.md](NAMING.md) — 命名决策日志

## License

双许可：[MIT](LICENSE-MIT) OR [Apache 2.0](LICENSE-APACHE)

数据集：CC-BY-NC 4.0 主库 + CC0 贡献 + CC0 quickstart 样本（见 [DATA.md](DATA.md)）。

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). We use **DCO** (Signed-off-by), not CLA.

## Related Projects

- **[VoxSign](https://voxsign.net)** — first consumer of perceptkit (voice assistant)
- **[SceneMind](https://github.com/smithpeter/scenemind)** — sister project, scene understanding methodology

## Acknowledgements

Inspired by Kahneman's dual-process theory, Mozilla Common Voice, Signal Protocol privacy model, HELM benchmark standards, and 1741 lines of hard-coded scene rules in VoxSign.
