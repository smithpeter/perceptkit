# perceptkit — Strategy (北极星文档)

> **一句话**：perceptkit 是 AI Agent 的**感知中间件**（Perception Middleware）——把多模态信号（音频 / 视觉 / 上下文 / 文本）转成可声明、可审计、可离线、可自学习的**场景决策（Scene Decision）**，让 agent 知道何时发声、何时倾听、何时升级到 LLM。
>
> Repo: `github.com/smithpeter/perceptkit` · License: `MIT OR Apache-2.0` · 首个消费者: VoxSign

**版本**: 1.0 · **生效日期**: 2026-04-18 · **经红蓝军三轮对抗确认（加权 7.25/10 ≥ 7.0 ship-worthy）**

---

## 1. 使命 · 定位

### 1.1 项目使命

让任何 AI Agent 能**准确、稳定、可持续地**识别真实世界的场景（包括从未见过的场景），不靠训练闭集分类器，不被单模态限制，不被 LLM 成本压垮。

### 1.2 市场定位

**Agent Context Layer**——LangChain / LlamaIndex / Haystack 生态里**缺失的那一格**：感知中间件。

| 生态格 | 代表 | perceptkit 关系 |
|---|---|---|
| 工具编排 (Tool Orchestration) | LangChain, CrewAI | 不竞争，可被上层调用 |
| 记忆 (Memory) | LangMem, Mem0 | 正交 |
| 检索 (Retrieval) | LlamaIndex | 正交 |
| **感知 (Perception)** | **空白** | **perceptkit 填充** |
| 模型路由 (Model Routing) | LiteLLM | 正交 |

### 1.3 产品一句话 Pitch

> *"The Perception Middleware for AI agents — declarative scenes, offline-first, self-learning."*

---

## 2. 战略五维（NorthStar）

**每个 PR / feature 必须证明至少一个维度增长。无增长 → 不做。** 这是治理，不是建议。

| 维度 | 权重 | v0.1 门 | v0.2 目标 | 关键度量 |
|---|---|---|---|---|
| **Accuracy 准确** | 25% | Top-1 ≥ 0.78 / ECE ≤ 0.12 | Top-1 ≥ 0.85 / ECE ≤ 0.10 | 标注 held-out 集 + per-scene recall ≥ 0.70 |
| **Learnability 自学习** | 25% | Qwen-0.5B 本地 Reflector 跑通 3 出口（Map / Propose / Unknown） | LocalReflector 生产质量 + Evolution Loop 闭环 | Unknown detection rate, Proposal acceptance rate |
| **Stability 稳定** | 20% | Flapping < 1/min, 降级不崩 | 同 v0.1 + 99.9% determinism | proptest + chaos test |
| **Performance 性能** | 15% | Hot path p95 < 5ms, core < 10MB, 纯 CPU | 同 v0.1 + hot p99 < 10ms | criterion + codspeed |
| **Adaptivity 自适应** | 15% | Modality enum + Reflector trait + Scene YAML | + Temporal / Stateful DSL | 新 Provider 接入 ≤ 1 day |

**维度分类**：
- **Floor（底线，不能跌破）**: Stability + Performance
- **Growth（持续投入）**: Accuracy + Learnability + Adaptivity

**决策四问**（每个 PR 必答）：
1. 本次改动主要增长哪个维度？
2. 是否削弱其他维度？
3. 净增长为正吗？
4. 能否测量维度增长？

没答出不进 main。

**战略 vs v0.1 门的落差**（诚实披露）：
- Accuracy 0.78 是 v0.1 基线，0.85 是长期目标。README "Known Limitations" 必须明说。
- 绝不偷偷降战略值——战略五维数字永不改，只改 v0.1/v0.2/v0.5 的阶段门。

---

## 3. 核心架构：Dual-Process (Kahneman 式双通道)

```
┌─ Signal Bus ────────────────────────────────────┐
│ Audio / Context / [Vision v0.2] / [Text v0.3]  │
└────────────────────┬────────────────────────────┘
                     ▼
┌─ Feature Layer (typed FeatureDescriptor) ──────┐
│ key + kind + unit + time_window + source + ver │
│ Levenshtein YAML 加载期校验 + 宏编译期校验     │
└────────────────────┬────────────────────────────┘
                     ▼
═══════════════ HOT PATH ═══════════════
│ RuleMatcher(YAML) + [EmbeddingMatcher v0.2]   │
│ → Arbiter → SceneHypothesis (conf, evidence)  │
═══════════════════════════════════════
                     ▼
            ┌── ConfidenceGate ──┐
            │ Accept / Escalate  │
            │ / Reject           │
            └────────┬───────────┘
                     ▼
═══════════════ COLD PATH ══════════════
│ Reflector (Noop / Mock / Qwen-0.5B 本地)      │
│ Tools: query_history, list_scenes, ...         │
│ 三出口: Map / Propose / Unknown                │
═══════════════════════════════════════
                     ▼
┌─ SceneDecision ──────────────────────┐
│ scene_id + conf + source + rationale │
│ + evidence + [proposed_scene?]       │
└──────────────┬───────────────────────┘
               ▼
┌─ Evolution Loop (带人工闸门) ────────┐
│ Pending → `perceptkit review` → YAML │
│ 永不自动 commit                       │
└──────────────────────────────────────┘
```

**七条不妥协原则**：
1. `perceptkit-core` 零 LLM / 零网络依赖（`cargo deny` 禁 `reqwest`/`tokio-net`）
2. LLM 是 tool-calling agent，不是 prompt-stuffing
3. 场景永远 YAML 外部化
4. 每个 SceneDecision 必须带 `rationale` + `evidence`
5. Reflection 有 budget（time / token / tool-calls）
6. 场景契约带 `version`，支持迁移
7. LLM 提议场景**永不自动入库**——`PendingSceneQueue` → 人工审核 → `scenes/*.yaml`

---

## 4. LLM 摊平威胁的 4 重答案（对抗"GPT-5-mini @ $0.0001/call"）

商业最致命质疑：**"LLM 便宜到 $0.0001/call 时，为什么还需要 Rust 规则引擎？"** 4 重答案落到架构：

| 答案 | 架构承载 | 度量 |
|---|---|---|
| **A. 离线/隐私/边缘** | `perceptkit-core` 零网络依赖，iOS/车载/IoT 永远本地 | `cargo test --no-default-features` + `cargo deny` |
| **B. 成本聚合** | `ConfidenceGate` 分流，Hot path p95 < 5ms，LLM 调用率 < 5% 样本 | `escalation_rate` runtime 指标 |
| **C. 确定性+可审计** | `SceneDecision.rationale` + `ReflectionTrace{prompt_hash, input, output_diff}` 持久化 JSONL | `perceptkit replay <trace>` 100% 确定性重放 |
| **D. Hybrid 才是答案** | `Reflector` trait 三实现 + `PendingSceneQueue` + `perceptkit review` CLI 把 LLM 建议变成 YAML PR | e2e: 低置信 → queue → LLM → 人审 → git commit |

perceptkit **不是替代 LLM**，而是**让 LLM 只在该用时用**。

---

## 5. 商业路径：D — VoxSign 壁垒 + 个人品牌

**接受路径 D**，不追商业化：

- **预算**: ≤ 3 人月（66 工作日 @ 22d/月）
- **融资**: 无
- **团队**: 作者单人（可接受 VoxSign dogfooding 反馈）
- **退出**: 2027-2028 被 Agent 平台 acquihire / 成为生态标配开源工具

**Moat 评估**（Round 2 Red Team 确认为"弱"，v0.1 不强求升级）：

| Moat 类型 | v0.1 | v0.2+ 提升路径 |
|---|---|---|
| 技术壁垒 | 弱-中（Rust+PyO3 门槛） | 中（temporal/stateful DSL 超 LangChain Router） |
| 网络效应 | 弱 | 中（社区贡献 scene YAML 库） |
| 生态绑定 | 弱 | 中（成为 `langchain_community.perception` 首发 adapter） |
| 数据壁垒 | 弱 | 中（perceptkit-bench 成为社区基准） |

**路径 D 成功标准**:
- VoxSign 1741 行 taxonomy ≥40% 可用 perceptkit YAML 表达（M7 强制审计）
- GitHub 1-3k star（12 个月内）
- 用户个人 staff/principal infra 面试信用 + 天使轮筹码

---

## 6. 非目标（v0.1 明确不做）

- ❌ 不做 ASR（调用方接 Whisper/SenseVoice）
- ❌ 不做 LLM 推理 in core（Reflector 是可插拔边界）
- ❌ 不做音频采集（只吃已解码 PCM ndarray）
- ❌ 不做持久化（SceneDecision 由调用方存）
- ❌ 不支持 streaming API（一次调用一次结果）
- ❌ 不支持 temporal DSL（`过去 30s 内 X 发生过` → v0.2）
- ❌ 不支持 stateful DSL（`如果上一场景是 Y 则...` → v0.2）
- ❌ 不支持 per-user override（→ v0.2）
- ❌ 不支持 Vision / Text modality（→ v0.3 / v0.4）
- ❌ 不追求商业化（路径 D 明确放弃）
- ❌ **永不做 Flywheel Telemetry**（见 §11 "Signal 模型"承诺，是 §4 A 离线承诺的极端实现）
- ❌ 不用 CLA，使用 DCO (Signed-off-by)（CLA 降贡献 35%，见 §11.7）

---

## 7. Roadmap

| 版本 | 时间 | 主题 | 代表能力 |
|---|---|---|---|
| **v0.1** | 2026 Q2 | Audio Foundation | Rule + Flapping FSM + Qwen-0.5B Reflector + Audio Provider + PyO3 + VoxSign POC |
| **v0.2** | 2026 Q3 | DSL Expressiveness | Temporal / Stateful / Per-user DSL + LocalReflector (llama-3B) 生产质量 |
| **v0.3** | 2026 Q4 | Vision | `perceptkit-vision` crate + MediaPipe 集成 |
| **v0.4** | 2027 Q1 | Text & Context | `perceptkit-context` + browser/window/motion signals |
| **v0.5** | 2027 Q2 | Benchmark as Standard | perceptkit-bench 成为社区基准 + LangChain 官方 adapter |

---

## 8. 与生态的关系

- **VoxSign** (`~/VoxSign`): 第一消费者，M5 POC，M7 Release 带 case study。VoxSign 1741 行 `edge/scene/` 替换为 perceptkit YAML。
- **SceneMind** (`~/projects/scenemind`): 场景理解方法论。perceptkit 实现 SceneMind 方法论的**工程侧**。README 交叉引用。
- **LangChain / LlamaIndex**: 可作为 `perception adapter`（v0.5 计划），不竞争编排层。
- **MediaPipe / Silero VAD / YAMNet**: perceptkit 可封装为 Provider，不重复造轮子。

---

## 9. 评审历史（红蓝军三轮对抗）

| Round | 裁决 | 加权分 | 关键结果 |
|---|---|---|---|
| R1 | 5/5 CONDITIONAL | - | 12 共识问题；Learnability=2/10 |
| R2 | Red CONDITIONAL | 5.25/10 | 5 致命漏洞（Accuracy 偷换/Reflector 空壳/60d 幻想/标注$150/VoxSign 5/N） |
| R3 | CONDITIONAL GO → GO | **7.25/10** | 3 决策（1-C / 2-A / 3-A）+ 2 微决策（$400 标注 + ≥40% 覆盖率） |

**战略由红蓝军锤过**，非作者一家之言。这是 perceptkit 治理的基石。

---

## 10. 决策权归属

- **战略五维数字** (权重 / 定义): 作者决定，红蓝军建议
- **v0.1/v0.2 门槛数字**: 作者决定，红蓝军必须投票
- **非目标清单**: 任何增项必须红蓝军 GO
- **商业路径变更** (D → B/C): 必须新一轮红蓝对抗
- **技术实现**: 作者自主，但需通过决策四问
- **LLM 提议场景入库**: **永远人工审核**，无例外

**此文件生效后，所有后续决策必须引用本文件章节号作为依据。**

---

## 11. 数据战略（经红蓝军 Round D1 对抗确认）

### 11.1 核心原则

**"数据比算法更重要"**（用户 2026-04-18 明确指示）。但在路径 D（不融资、不商业化）约束下，数据战略必须避免两个陷阱：

1. **Common Voice 悖论** — 无商业闭环的开源数据，规模再大也不值钱（Mozilla 7 年 2800h，估值归零）
2. **Privacy Theater 陷阱** — Differential Privacy 对个人项目不现实（Round D1 架构视角：Apple DP 团队 20+ 人，Carlini 2020/2023 证 embedding inversion 可恢复 80%+ 语义）

### 11.2 两层架构（Layer 3 明确放弃）

```
Layer 1 — Seed Dataset (v0.1, HF 托管)
  └─ 525 片段, speaker-disjoint split, synthetic <30%, kappa ≥0.70
Layer 2 — Community Contribution (v0.2)
  └─ `perceptkit contribute` CLI, DCO (Signed-off-by), CI 校验
Layer 3 — [明确砍掉] Flywheel Telemetry
  └─ 遵循 "Signal 模型"：perceptkit 二进制零 network call
     是 §4 A 离线承诺的极端实现
```

### 11.3 "Signal 模型"承诺（§4 A 强化）

perceptkit **永不编译进 telemetry crate**，永不在二进制里埋 network call。用户想贡献数据必须：

1. 明确执行 `perceptkit export --case <id>` 生成人类可读的 YAML + metadata
2. 眼睛看过内容
3. 主动 git PR 到 `perceptkit-community/bench`（DCO 签署）

这比 DP 更强的信任承诺——"我们连采集的能力都没有"。

### 11.4 数据许可（B2 决策）

| 层 | 许可 | 理由 |
|---|---|---|
| 主数据集 `perceptkit-bench-v0` | **CC-BY-NC 4.0** | 阻止商业克隆，保 VoxSign 壁垒 |
| 贡献数据（DCO 要求） | **CC0** | 无摩擦合并，可入主库 |
| Quickstart 样本（100 片段） | **CC0** | 降试用门槛，可 embed README |

### 11.5 VoxSign 数据处置（D2+D1 混合）

- ❌ VoxSign 真实用户数据**不进入** perceptkit 公开 pipeline
- ✅ VoxSign 数据仅作**内部 rule-tuning** + **离线 baseline 评估**
- ✅ perceptkit 公开 benchmark 只用 AudioSet / MUSAN / UrbanSound8K 公开数据 + <30% 合成混合数据（仅入 train, held-out 纯真实）

**理由**:
- 架构视角警告 "VoxSign 是身份认证场景，一次泄露 = 职业生涯终结"
- QA 视角警告循环污染（VoxSign 用 perceptkit → 贡献回 perceptkit = 循环评估）
- 商业视角：VoxSign 数据是真 Moat，开源反而稀释

### 11.6 数据质量门（QA 视角 5 要求全采纳）

所有贡献必须过 `perceptkit lint` + CI：

1. **Speaker-disjoint split** — 同一说话人声纹不能跨 train/test（防 accuracy 虚高 10-20%）
2. **Kappa 漂移监控** — 每月 5% 抽样三标注员复核，整体 kappa <0.65 冻结版本
3. **数据集版本化** — `perceptkit-bench-v{major}.{minor}`，v0 永 frozen 作 legacy baseline
4. **类别长尾** — 单类 <30 样本不入评估，首要指标 **macro-F1**（不是 Top-1，会被头部类主导）
5. **对抗样本防护** — 首贡 100% 人审 + 之后 10% 抽审 + reputation score

### 11.7 数据集治理

- **仓库**: `smithpeter/perceptkit-bench-v0` 独立 repo（v0.1-v0.3），v0.4 视社区规模迁 `perceptkit-community/bench` 独立 org
- **托管**: HuggingFace Datasets 主渠道 + **S3/IPFS mirror**（HF revision 非 immutable）+ **sha 公证**锁在 GitHub Release
- **贡献协议**: **DCO** (Signed-off-by)，不是 CLA
  - 产品数据：Kubernetes DCO vs CLA A/B 显示 CLA 降首贡 35%
  - 形式：每个 commit 加 `Signed-off-by: Name <email>`，作者 DCO v1.1 声明
- **审核**: 首贡 100% 人审（reputation 0）→ 3+ 贡献后进入 10% 抽审 → 违规回到 100% 人审

### 11.8 三件套文档（HELM 标准）

独立 `DATA.md` + 版本化（见仓库 `DATA.md`）：

- **Datasheet** — 数据集描述（规模 / 来源 / 采集过程 / 人口学 / 许可 / limitations）
- **Model Card** — 每个 release 的模型/规则能力 + 限制 + biases
- **Eval Card** — 评估流程 / 指标定义 / 基线对比 / 复现命令

### 11.9 Moat 现实主义

路径 D 下，数据飞轮 ROI 为负（商业视角 Round D1）。数据战略的**真实价值**:

- ❌ 不是升 Moat 到"中"（路径 D 下做不到）
- ✅ **建立可信度** (credibility) — 让 repo 在 HN/VC/雇主眼中不是"玩具"
- ✅ **为 VoxSign 背书** — "我们的感知层有 525 片段 labeled benchmark"
- ✅ **个人面试信用** — HF dataset download 比 GitHub star 更 sticky

**接受**：perceptkit 最可能的终点是 Silero VAD 式（5-10k star，$0 ARR，被白嫖但赢得技术声誉）。这和路径 D 目标一致。

**参考本文件最终版本由红蓝军数据战略 Round D1 对抗（2026-04-18）确认。未来变更须重新对抗。**
