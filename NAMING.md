# perceptkit — Naming Decision Log

> 为什么叫 perceptkit？给后来者 / 贡献者 / 未来考古学家的解释。
>
> 本文件不可改写历史——如果未来改名，追加新章节，不删旧章节。

---

## 命名演化史

```
SceneKit (2026-04-18 初版)
   ↓ 被红蓝军 Round 1 否决（Apple SceneKit 冲突）
Percept (2026-04-18 Round 2 候选)
   ↓ 尽职调查发现 PyPI 僵尸包
perceptkit (2026-04-18 最终确定)
```

---

## Round 1: 为什么不叫 SceneKit

### 冲突源
**Apple SceneKit** — macOS / iOS / tvOS / watchOS 的 3D 图形框架，自 2012 年 OS X 10.8 推出，至今是 Apple 原生 3D 图形的官方 API。

### 问题
- **SEO 被压**: `google "scenekit"` 前 50 条全部是 Apple 文档、WWDC 视频、Stack Overflow Swift 问答
- **商标风险**: Apple 对 `*Kit` 系列命名有强品牌主张（UIKit, SwiftUI, ARKit, HealthKit...）
- **品牌混淆**: 开发者看到 "SceneKit" 会先想 3D，不会联想场景识别

### 产品经理原话（Round 1）
> "SceneKit 最大问题——Apple 有 SceneKit（3D 框架），商标 / SEO 直接被压。"

---

## Round 2: 为什么改 Percept 后又改 perceptkit

### 候选 Percept 的优点
- 单词干净、语义直指"感知"
- crates.io 可用 (404)
- GitHub `smithpeter/percept` 可用 (404)

### Percept 的两个阻塞

#### 阻塞 1: PyPI 僵尸包占用
- 2013 年 Equirio 用户发布 `percept` 0.14 ML 框架（AGPL）
- 最后更新 **2013-07-09**，12 年未更新
- 走 PEP 541 reclamation 流程需 3-6 个月，可能被拒
- 无法在 v0.1 发布时间线内释放

#### 阻塞 2: Microsoft Azure Percept 品牌残留
- Azure Percept 2021 上线（边缘 AI 平台，含 Percept Vision / Percept Audio）
- 2022-03 停售（product discontinued）
- **问题**: 与本项目"感知中间件"定位**领域重叠**
- 虽产品已停，但搜索引擎记忆尚存 2-3 年
- 直接用 Percept 会和 Azure Percept 文档混淆

### 为什么选 perceptkit

**优点**:
- ✅ crates.io: 可用 (404)
- ✅ PyPI: 可用 (404)
- ✅ GitHub smithpeter/perceptkit: 可用 (404)
- ✅ `-kit` 后缀避开 Azure Percept 精确匹配
- ✅ 符合工具库命名惯例: `tiktoken` / `diffkit` / `langkit` / `fasttext-kit`
- ✅ 保留 "percept" 语义核心
- ✅ 域名友好: `perceptkit.dev` (待注册检查) / `perceptkit.io`

**品牌风险接受**:
- Azure Percept 搜索污染：`-kit` 后缀分化足够
- `SceneKit` Apple 冲突：perceptkit 名字不含 Scene，内部 API 叫 `Scene` 可接受

---

## 术语表（统一对外对内）

| 术语 | 中文 | 定义 | 例子 |
|---|---|---|---|
| **Scene** | 场景 | 用户所处的情境状态 | `online_meeting`, `driving` |
| **Signal** | 信号 | 原始输入（带时间戳 + 模态） | audio PCM, window.app event |
| **Feature** | 特征 | 从信号提取的度量 | `audio.voice_ratio = 0.62` |
| **FeatureDescriptor** | 特征描述符 | Feature 的类型元数据（key / kind / unit / window） | — |
| **FeatureBundle** | 特征包 | 某一时刻所有 Features 的集合 | — |
| **SceneEngine** | 场景引擎 | 核心入口，加载 scenes/ + 执行评估 | `SceneEngine::from_dir(...)` |
| **Arbiter** | 仲裁器 | Hot path 终端：多候选 → SceneHypothesis | — |
| **ConfidenceGate** | 置信闸门 | 决定是否走 Cold Path | — |
| **Reflector** | 反思器 | Cold path LLM agent | `NoopReflector` / `LocalReflector` |
| **Reflection** | 反思结果 | 三出口：Map / Propose / Unknown | — |
| **PendingSceneQueue** | 待审场景队列 | LLM 提议入库前的暂存 | SQLite table |
| **SceneDecision** | 场景决策 | 最终输出（scene_id + conf + rationale + source） | — |
| **ReflectionTrace** | 反思轨迹 | LLM 调用的可回放 JSONL 记录 | — |

**注意**: 不用 `Episode`（Round 3 审官建议，但有 RL 语义污染）。不用 `Situation`（太冗长）。**`Scene` 就是 `Scene`**。

---

## crate / 包命名

| 发布渠道 | 名称 | 说明 |
|---|---|---|
| Rust crates.io | `perceptkit-core` | 核心 trait + DSL |
| Rust crates.io | `perceptkit-audio` | 音频 Provider |
| Rust workspace (internal) | `perceptkit-py` | PyO3 binding（不发 crates.io） |
| PyPI | `perceptkit` | 整合 Python 包 |
| GitHub | `smithpeter/perceptkit` | monorepo |
| HuggingFace Datasets | `smithpeter/perceptkit-bench-v0` | 标注数据集 |
| 命令行工具 | `perceptkit` | CLI binary |

---

## 未来改名触发条件（不建议轻易改）

- 只在以下情况重启命名讨论：
  - 商标方（Apple / Microsoft）主动发送 cease-and-desist
  - 项目定位根本转向（如不再做 perception，转 orchestration）
  - 社区强烈要求（GitHub issue 获 ≥100 👍）

- **不改名理由**（历史积累价值）：
  - crates.io / PyPI 下载量
  - GitHub star 历史
  - 博客 / HN / Reddit 反向链接 SEO
  - 搜索引擎索引
