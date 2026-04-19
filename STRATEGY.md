# perceptkit — Strategy (北极星文档)

> **一句话**：perceptkit 是 AI Agent 的**感知中间件**（Perception Middleware）——把多模态信号（音频 / 视觉 / 上下文 / 文本）转成可声明、可审计、可离线、可自学习的**场景决策（Scene Decision）**，让 agent 知道何时发声、何时倾听、何时升级到 LLM。
>
> Repo: `github.com/smithpeter/perceptkit` · License: `MIT OR Apache-2.0` · 首个消费者: VoxSign

**版本**: 2.0 · **生效日期**: 2026-04-19 · **数据飞轮元假设激活（红蓝军 Round 5/6 对抗确认）**

> **v2.0 重大变更**：用户元假设"数据是终极护城河"激活，触发 1C+2B+3C 战略大转向。Signal Model 从"项目级红线"降级为"部署级选择"；引入 dual form factor（core 嵌入式版 + cloud 数据飞轮版）；新增旗舰 vertical（知识工作者会议场景）；VoxSign 关系从"代码独立"扩展为"代码独立 + 数据流通"。详见 §11（重写）+ §12（新）+ §13（新）。

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

**Moat 评估**（v2.0 重排：数据壁垒升为主路径）：

| Moat 类型 | v0.1 | **v0.2+ 主路径**（v2.0 重排） |
|---|---|---|
| **数据壁垒** ⭐ | 弱 | **强**（旗舰 vertical 数据飞轮，见 §11+§12+§13） |
| 技术壁垒 | 弱-中（Rust+PyO3 门槛） | 中（temporal/stateful DSL + dual form factor） |
| 生态绑定 | 弱 | 中（vertical bench 成为该领域 canonical eval） |
| 网络效应 | 弱 | 中（社区贡献 scene YAML 库 + dataset PR） |

**v2.0 元假设**（来自用户）：算力商品化、算法商品化、**数据稀缺**。因此长期 Moat 必须围绕"独家高质量数据资产"。详见 §11。

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
- ❌ ~~永不做 Flywheel Telemetry~~ **[v2.0 修订]** 详见 §11+§12：`perceptkit-core/audio/py` 永不做（嵌入式/合规承诺保留），但新增 `perceptkit-cloud` opt-in 模块支持自愿数据回流（数据飞轮入口）
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

## 11. 数据战略（v2.0 — 数据飞轮元假设）

### 11.1 核心原则

**用户元假设（2026-04-19）**："数据是终极护城河。算力、算法都比较容易买到。核心是积累数据。"

这是**项目元决策**，所有其他决策让位于此。具体推论：

- 算力商品化 → Silero / YAMNet / Qwen / Whisper 都"几乎免费"
- 算法商品化 → 论文实现 6-12 个月内必有 OSS 跟进
- **数据稀缺** → 高质量、有标注、领域 specific 的多模态场景数据**真的买不到**
- 因此 perceptkit 的长期价值必须围绕"积累独家数据资产"构建

### 11.2 三层架构（v2.0：Layer 3 复活为 opt-in cloud）

```
Layer 1 — Seed Dataset (v0.1 已 ship)
  └─ 公开数据 + 合成 + 525 片段标注，建立 baseline + 可信度

Layer 2 — Community Contribution (v0.2)
  └─ `perceptkit contribute` CLI + DCO + CI 校验，社区共建公开数据

Layer 3 — Data Flywheel (v0.2 新增) ⭐
  └─ perceptkit-cloud crate（独立模块，opt-in feature）
     用户/合作方主动启用 → 脱敏 trace 上传 → perceptkit-bench-internal
     筛选审核后部分公开为 perceptkit-bench-v1
     【core/audio/py 仍然零网络，cloud 是独立模块，详见 §12】

Layer 4 — Vertical Data Asset (v0.2 新增) ⭐⭐
  └─ perceptkit-bench-knowledge-work：旗舰 vertical 数据壁垒
     VoxSign 脱敏数据 + 合作方数据 + 公开数据
     目标：成为"知识工作者会议场景"领域 canonical benchmark
     详见 §13
```

### 11.3 Signal Model 修订：从"项目红线"降级为"部署选择"

**v1.0 旧定义（已废弃）**：perceptkit 二进制永不有 network call。
**v2.0 新定义**：

| Crate | network 默认 | 工程保证 |
|---|---|---|
| `perceptkit-core` | **永远零网络** | `cargo deny` 持续封 reqwest/hyper/etc |
| `perceptkit-audio` | **永远零网络** | 同上 |
| `perceptkit-py` | **永远零网络** | 同上 |
| **`perceptkit-cloud`**（新） | opt-in，默认 off | 独立 crate，独立 SBOM，**用户必须显式 build with `--features cloud`** 才能开启 |

**用户承诺重写**：
> *旧*: "We can't collect your data because we don't have the ability to."
> **新**: "Embedded mode (default) cannot collect your data. Cloud mode (opt-in) can — and we tell you exactly when, what, where, and let you turn it off any time."

这给两个市场：
- **嵌入式/合规客户**（医疗/法律/车载）: 用 core，零网络，承诺不变
- **数据飞轮参与者**（OSS 开发者 / 合作伙伴 / VoxSign）: 用 cloud，opt-in 贡献数据换取更准确的 vertical 模型

### 11.4 数据许可（沿用 v1.0 决策）

| 层 | 许可 | 理由 |
|---|---|---|
| `perceptkit-bench-v0`（公开）| **CC-BY-NC 4.0** | 阻止商业克隆 |
| 贡献数据（DCO 要求）| **CC0** | 无摩擦合并 |
| Quickstart 样本（100 片段）| **CC0** | 降试用门槛 |
| **`perceptkit-bench-internal`**（v0.2 新增）| **私有，VoxSign 控制** | 数据飞轮原始素材 |
| **`perceptkit-bench-knowledge-work-v0`**（v0.2 新增）| CC-BY-NC | 旗舰 vertical 公开版 |

### 11.5 VoxSign 数据处置（v2.0 修订：D2 → 数据流通）

**v1.0 旧**：~~VoxSign 真实用户数据**不进入** perceptkit 公开 pipeline~~
**v2.0 新**：

```
VoxSign 用户 (opt-in EULA)
  ↓ 产生原始 trace
voxsign-anonymizer (VoxSign 仓内独立工具)
  ├─ 删除 PII / 声纹 hash / 时间戳粗化 / 文本 GPT 改写
  └─ 法务+技术双签批准
  ↓ 脱敏聚合数据
perceptkit-bench-internal (私有仓, VoxSign 控制)
  ↓ 训练默认 rules + tuning vertical scenes
perceptkit-bench-knowledge-work-v0 (公开仓, CC-BY-NC)
  └─ 仅人工挑选样本 + 二次脱敏 + 双签
```

**红线**：
- ❌ 原始音频/文本/PII **永不**直接进 public 仓
- ❌ 单 trace **永不**直传，最小聚合粒度 = 100 同类场景
- ❌ VoxSign 客户数据被反向 codify 进 perceptkit 默认 scenes（保通用性）
- ✅ 用户 opt-out 可立即停止后续贡献，已贡献数据保留

### 11.6 数据质量门（沿用 v1.0 5 要求 + v2.0 vertical 扩展）

所有贡献必须过 `perceptkit lint` + CI，新增 vertical-specific：

1. Speaker-disjoint split（防 accuracy 虚高 10-20%）
2. Kappa 漂移监控（每月 5% 抽样三标注员复核）
3. 数据集版本化（v{major}.{minor}，老版永 frozen）
4. 类别长尾保护（首要指标 macro-F1）
5. 对抗样本防护（首贡 100% 人审 + reputation）
6. **【v2.0 新增】Cross-vertical contamination check** — knowledge_work bench 不能掺入 driving / outdoor 等其他 vertical 数据
7. **【v2.0 新增】Anonymization audit** — VoxSign 流入数据每季度抽样反向工程检查 (re-identification rate < 1%)

### 11.7 数据集治理（沿用 v1.0）

- **仓库**: `smithpeter/perceptkit-bench-v0`（v0.1）+ `smithpeter/perceptkit-bench-knowledge-work-v0`（v0.2）独立 repo
- **托管**: HuggingFace Datasets 主渠道 + S3/IPFS mirror + sha 公证锁 GitHub Release
- **贡献协议**: DCO（Kubernetes A/B 显示 CLA 降首贡 35%）
- **审核**: 首贡 100% 人审 → 3+ 贡献后 10% 抽审 → 违规回到 100%

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

### 11.9 Moat 升级路径（v2.0 修订）

**v1.0 旧（已过时）**：~~"路径 D 下数据飞轮 ROI 为负，数据战略只是建立可信度"~~

**v2.0 新**（基于用户元假设激活）：
- **数据飞轮是主路径**，不是辅助
- 即使路径 D 不融资，**vertical 数据资产**本身就有商业转让价值（acquihire 时数据是核心资产，不是代码）
- 接受度量：`perceptkit-bench-knowledge-work-v0` ≥ 200h labeled vertical data + ≥ 1 数据合作 LOI = "数据壁垒"维度从弱升中

**红蓝军 Round 5/6（2026-04-19）确认**：v1.0 数据战略在数据飞轮元假设下不成立，必须改写为 1C+2B+3C 组合。本 §11 v2.0 是新版本基线。未来变更须重新对抗。

---

## 12. Dual Form Factor（v2.0 新增）

**1C 决策**：perceptkit 同时服务两个对立市场，靠**模块切分**而非"折中设计"。

### 12.1 形态 A — Embedded / Compliance Mode

**Crates**: `perceptkit-core` + `perceptkit-audio` + `perceptkit-py`

**承诺**:
- 零网络（cargo deny 持续封 reqwest/hyper/surf/ureq/awc）
- 零 telemetry
- 零强制依赖外部服务
- SBOM 可审计，每个 release 附 `egress_audit.txt`

**目标客户**: 医疗 / 法律 / 车载 / IoT / 隐私敏感个人开发者

**叙事**: *"We can't collect your data. The binary doesn't have the ability."*

### 12.2 形态 B — Cloud / Data Flywheel Mode

**Crate**: `perceptkit-cloud`（v0.2 新增，独立 crate，独立 SBOM）

**默认状态**: 不编译，不安装，不存在于 default workspace
**启用方式**: `cargo add perceptkit-cloud` + 显式 `--features cloud` + 用户 init 时 explicit opt-in prompt

**功能**:
- `TelemetryReporter` — opt-in trace 上传（sanitized SceneDecision 只含 scene_id + confidence + timestamp，**不含原始 features 或 PCM**）
- `DatasetUploader` — opt-in 完整 trace 贡献（带原始 features，**用户每次确认**，附 sanitization preview）
- `RemoteReflector` — 可选远程 LLM Reflector（替代 LocalReflector，需 user-provided API key）

**目标用户**: 数据飞轮参与者（VoxSign / 合作 OSS / 主动贡献者）

**叙事**: *"You opt in, we tell you exactly what flows out, you can stop anytime, you get back better-tuned vertical scenes."*

### 12.3 工程隔离保证

- `perceptkit-cloud` **永远** depends on `perceptkit-core`，反向**永不**
- core/audio/py 的 `cargo deny` 不放宽（cloud 在自己 deny.toml 管理）
- CI 矩阵保留 "default profile（无 cloud）" 和 "cloud profile" 双跑
- README 双 quickstart：embedded 在前（默认推荐），cloud 在 §"Contributing your data"

### 12.4 失败模式预防

- ❌ 不允许 core 暴露 hidden hook 让 cloud 旁路 opt-in（架构层 trait boundary 严格）
- ❌ 不允许 cloud 反向修改 core API 让 telemetry 渗入默认路径
- ❌ 不允许"为方便"打包 cloud 进 default wheel
- ✅ Allowed: 用户主动 `pip install perceptkit[cloud]` 显式安装

---

## 13. Vertical Anchor — Knowledge Worker Meeting Scenes（v2.0 新增）

**3C 决策**：通用框架（拉社区） + **1 个旗舰 vertical**（建数据壁垒）。

### 13.1 vertical 选定理由

锁定 **"知识工作者会议场景"**（knowledge_work），因为：

1. **VoxSign 已有数据** — 数千小时真实会议音频 + 用户 opt-in 协议成熟（数据飞轮启动 day 1）
2. **市场已验证** — Granola / Superwhisper / Wispr Flow / Otter / Krisp 都在这赛道，用户付费意愿验证过
3. **TAM 够大** — 全球知识工作者 ~1B，活跃使用 AI 会议工具 ~50M，5 年 TAM ≥ $20B
4. **数据稀缺真实** — 真实会议数据被各家锁起来，公开高质量 labeled 数据 < 50h（ICSI Meeting Corpus + AMI ≈ 100h 但口音单一年代久）
5. **多模态需求强** — 音频 + 屏幕 app + 日历 + 麦克风类型 + 网络状态，正是 perceptkit Dual-Process 设计目标

### 13.2 vertical 场景库（v0.2 ship）

预置 10 个 scene YAML（区别于 v0.1 通用 5 场景）：

| scene_id | 触发信号 | 用途 |
|---|---|---|
| `meeting_zoom` | context.app=Zoom + audio.voice_ratio>0.4 + audio.speaker_count>=2 | 主流程会议 |
| `meeting_huddle` | context.app=Slack + huddle_indicator | 临时讨论 |
| `interview_conducting` | meeting + 1 dominant speaker + scheduled "interview" calendar | 面试模式 |
| `standup` | <15min + 3-8 speakers + recurring | 站会 |
| `brainstorm` | meeting + high speaker_transitions + low silence | 头脑风暴 |
| `1on1` | exactly 2 speakers + ≥20min + recurring | 1对1 |
| `pair_coding` | meeting + IDE app foreground + low audio | 结对编程 |
| `focus_writing` | no meeting + writing app foreground + low ambient | 专注写作 |
| `async_recording` | recording app + 1 speaker + no meeting | 异步录制 |
| `phone_call` | telephony + 1-2 speakers + non-meeting app | 电话 |

每个带 rationale + evidence schema，支持 LocalReflector "Map / Propose / Unknown"。

### 13.3 vertical bench 目标

`smithpeter/perceptkit-bench-knowledge-work-v0`：

| 维度 | v0.2 目标 | v0.3 目标 |
|---|---|---|
| 总时长 | ≥ 200h labeled | ≥ 1000h |
| 来源多样性 | ≥ 3 数据源 (VoxSign + 公开 + 合作) | ≥ 5 |
| 覆盖 scenes | 10 个全覆盖，单类 ≥ 30 | 单类 ≥ 100 |
| 标注 kappa | ≥ 0.75（vertical 比通用容易达到） | ≥ 0.80 |
| Macro-F1 baseline | ≥ 0.70 | ≥ 0.85 |

### 13.4 vertical 不做什么（防 scope creep）

- ❌ 不做实时转写（调用方接 Whisper）
- ❌ 不做摘要 / action item（上层 agent 做）
- ❌ 不做发言人识别（敏感 + 可被替代）
- ❌ 不做情感分析（v0.4+ 评估）
- ❌ 不固化为只服务 VoxSign 的 schema（保 vertical 通用性）

### 13.5 通用框架的存在理由

vertical anchor 之外仍保留通用 framework，因为：
- 通用 framework 拉社区贡献其他 vertical（healthcare consultation / legal deposition / driving / smart home）
- vertical 是"打样间"，通用是"操作系统"
- 没有通用框架则 vertical 退化为 SaaS 式垂直产品，丧失 OSS 杠杆

---

## 14. v2.0 治理：决策审计

v1.0 → v2.0 触发：用户元假设"数据是终极护城河"激活（2026-04-19）。

| 章节 | v1.0 → v2.0 变化 | 理由 |
|---|---|---|
| §6 非目标 | "永不 telemetry" 改为 "core 永不、cloud 可选" | 1C dual form factor |
| §11 数据战略 | 全章重写：Layer 3 复活，VoxSign 数据接入 | 数据飞轮元假设 |
| §11.5 VoxSign | "不进 pipeline" 改为 "脱敏后入 internal 池" | 2B 代码独立 + 数据流通 |
| §11.9 Moat | "可信度" 改为 "数据壁垒主路径" | 用户元假设直接推论 |
| §12 (新) | dual form factor 显式定义 | 1C 落地 |
| §13 (新) | vertical anchor 显式定义 | 3C 落地 |

**v2.0 由红蓝军 Round 5/6（2026-04-19）对抗确认**。后续变更须新一轮对抗。
