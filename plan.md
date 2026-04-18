# perceptkit — Implementation Plan (v0.1)

> 执行蓝图，经红蓝军 3 轮对抗最终确认。本文件与 `STRATEGY.md` 配套——战略由 STRATEGY 定，执行由 plan 定。
>
> **预算**: 66 工作日（3 人月 @ 22d/月），含 2d 缓冲
> **原则**: 每个 milestone 标注目标维度，DoD 可自动验证，D45/D55/D60 三级砍刀硬约束

---

## 0. 总览

| M | 名称 | 天数 | 目标维度 | 关键产出 |
|---|---|---|---|---|
| M1 | 骨架 & 命名 | 3 | +Stability | workspace + CI + serde_yml + LICENSE |
| M2 | Core + Dual-Process 骨架 | 12 | +Learnability +Adaptivity +Stability | Arbiter/Gate/Reflector trait + typed FeatureDescriptor + scene lint |
| M3 | Audio + Flapping FSM | 8 | +Accuracy +Stability | RMS/VAD + Hysteresis FSM + proptest |
| M4 | Bench + Dataset v0 | 9 | +Accuracy | 525 片段 + kappa 0.70 + CI gate |
| M5 | PyO3 abi3 binding | 7 | +Performance | numpy zero-copy + .pyi + wheel CI |
| M6 | Reflector: Noop+Mock+Qwen-0.5B | 13 | +Learnability +Adaptivity | 本地 LLM 真跑 3 出口 + VCR fixture |
| M7 | VoxSign POC + DATA.md + Release | 13 | 全维 | 5 YAML + 覆盖率审计 + 数据三件套 + Signal 模型 SBOM + 博客 + HN |
|   | **合计** | **65d + 1d 缓冲 = 66d** |   |   |

**Checkpoints（硬砍刀，STRATEGY.md §5 路径 D 的 kill-switch）**：

- **D45**: 必须过 M4（数据集 kappa 门 + CI gate 落地）
  - 未过 → **砍刀 L1**: 砍 M7 HN/博客到 v0.1.1，v0.1 做 silent release
- **D55**: 必须过 M5 + M6 Noop
  - 未过 → **砍刀 L2**: 砍数据集 525 → 150 片段（标为 v0.1 baseline，v0.2 扩）
- **D60**: 必须过 M6 Qwen 集成
  - 未过 → **砍刀 L3**: 砍 PyO3 abi3 到 py311 专属 wheel（CI 矩阵减半）
- **任意时刻**: perceptkit 不得让 VoxSign 主线延误超 60d（硬红线）

---

## 1. M1 — 骨架 & 命名体系 (3d, +Stability)

### 1.1 产出
- Cargo workspace: `perceptkit-core` / `perceptkit-audio` / `perceptkit-py`
- `pyproject.toml` + maturin (`manifest-path = "crates/perceptkit-py/Cargo.toml"`)
- 双许可: `LICENSE-MIT` + `LICENSE-APACHE`
- `README.md` (英 + 中 section)
- GitHub Actions: fmt / clippy / `cargo deny` / pytest
- `cargo deny` 规则: 禁 `reqwest` `tokio-net` 在 core（离线保证）
- Repo push: `github.com/smithpeter/perceptkit`

### 1.2 关键技术选择
- YAML 解析: **`serde_yml`** (非 archived `serde_yaml`)
- 错误类型: `thiserror`
- 日志: `tracing`
- Python ABI: `abi3-py311`（兼容 3.11+）

### 1.3 DoD ✅ 2026-04-18 完成（CI run 24603823xxx 全绿）
- [x] `cargo build --workspace` 绿
- [x] CI matrix: macOS arm64 + Linux x86_64 过
- [x] `cargo deny check` 在 core 阻止网络 crate（Signal 模型工程保证）
- [x] `maturin develop` 可 import 空 Python 包（4 matrix: macOS/Linux × py311/py312）
- [x] README 含一句话 pitch + quickstart 骨架
- [x] DCO hook (`.githooks/commit-msg`) + GitHub Actions DCO job
- [x] 首次 commit DCO 签名（`Signed-off-by: Yongming Zou`）

---

## 2. M2 — Core + Dual-Process 骨架 (12d, +Learnability +Adaptivity +Stability)

这是 v0.1 的**最核心里程碑**。Round 2 Red Team 指出 "plan↔战略脱节"——M2 必须一次性把 Dual-Process 落地，不是 stub。

### 2.1 关键 Trait（Rust 签名）

```rust
// hot path - 同步
pub trait Arbiter: Send + Sync {
    fn decide(&self, ctx: &EvalCtx, matches: &[SceneMatch]) -> SceneDecision;
}

pub trait ConfidenceGate: Send + Sync {
    fn verdict(&self, d: &SceneDecision, ctx: &EvalCtx) -> GateVerdict;
}
pub enum GateVerdict { Accept, Escalate { reason: String }, Reject }

// cold path - async
#[async_trait]
pub trait Reflector: Send + Sync {
    async fn reflect(&self, case: PendingCase) -> Result<Reflection, ReflectError>;
    fn name(&self) -> &'static str;
    fn fingerprint(&self) -> PromptHash;
}
pub enum Reflection {
    Map { scene_id: String, rationale: String },
    Propose { yaml: String, examples: Vec<PendingCase> },
    Unknown { metadata: FeatureSummary },
}

// Feature 类型系统（解决 Round 1 "扁平 HashMap 欠工程"）
pub struct FeatureDescriptor {
    pub key: FeatureKey,           // newtype, 验证 "a.b.c" 格式
    pub kind: FeatureKind,         // F64{min,max} | Bool | Category | Vec{dim}
    pub unit: Option<&'static str>,
    pub window: TimeWindow,        // Instant | Sliding(Duration) | Ema{alpha}
    pub source: Cow<'static, str>, // "perceptkit-audio@0.1.0"
    pub version: u32,
}

pub enum FeatureKind {
    F64 { min: Option<f64>, max: Option<f64> },
    Bool,
    Category(Vec<&'static str>),
    Vec { dim: usize },
}
```

### 2.2 Scene YAML DSL (v0.1)

```yaml
id: online_meeting
version: 1
describe:
  template: "{participants}人在线会议 ({app})"
match:
  all:
    - { feature: audio.voice_ratio, op: gt, value: 0.4 }
  any:
    - { feature: context.app, op: in, value: [zoom.us, Teams, Feishu] }
    - { feature: audio.speaker_count, op: gte, value: 2 }
priority: 10
```

### 2.3 FeatureRegistry 校验（编译期 + 加载期双层）

- **编译期**: `feature!("audio.voice_ratio")` 宏查静态注册表，typo 直接编译失败
- **加载期**: `SceneEngine::from_dir` 扫所有 YAML `feature:` 引用 → 与 Registry 求差集 → 未注册 → `ConfigError::UnknownFeature { key, did_you_mean: Levenshtein 提示 }`

### 2.4 PendingSceneQueue
- SQLite 单表
- 字段: `id / created_at / trigger_case_jsonl / proposed_yaml / status(pending/approved/rejected) / reviewer / reviewed_at`
- CLI: `perceptkit review list` / `perceptkit review approve <id>` / `perceptkit review reject <id>`

### 2.5 perceptkit lint
- 冲突检测: 扫所有 YAML 条件交集 → 报告任意 FeatureBundle 命中 > 1 场景且无 priority 裁决
- 覆盖度: held-out 样本被命中率 ≥ 95%
- Feature 引用校验（同 §2.3）

### 2.6 DoD ✅ 2026-04-18 完成（CI run c7ef72b 全绿，38 tests）
- [x] Arbiter / ConfidenceGate / Reflector trait 编译通过
- [x] `FeatureDescriptor` typed + Levenshtein 提示（`feature!` 宏延后，加载期校验已就位）
- [x] NoopReflector 可跑（返回 `Unknown`）
- [x] YAML typo `voice_ratios` → 加载期报错 + 建议 `voice_ratio` (test_registry::resolve_or_error_gives_did_you_mean)
- [x] `perceptkit lint` 对 test fixture 能检出构造的冲突场景 (test_engine::lint_detects_overlapping_priority_conflict)
- [x] 38 单元测试全绿（覆盖率 ≥ 70% 目标达成）
- [x] 5 starter scenes YAML（office_quiet / online_meeting / driving / outdoor_noisy / multi_speaker_chat）

---

## 3. M3 — Audio Provider + Flapping FSM (8d, +Accuracy +Stability)

### 3.1 Audio Extractors
- `EnergyExtractor`: RMS / peak / SNR（谱减法估计）
- `VoiceActivityExtractor`: 能量 + 过零率（v0.1 简化，v0.2 接 Silero ONNX）
- `MultiSpeakerExtractor`: voice_ratio（v0.1 预留 trait，实现留 stub）
- `SoundEventExtractor`: trait 定义，实现延后（v0.2 接 BEATs/YAMNet）

### 3.2 Flapping FSM
**参数默认**:
- `enter_hi = 0.70`（进入阈值）
- `exit_lo = 0.55`（退出阈值）
- `dwell = 3s`（最小驻留时间）
- `hot_switch_floor = 0.85`（紧急场景跳过 dwell）

**状态**: `Stable(s)` / `Candidate(s_new, since)` / `Uncertain`

**规则**:
1. 当前 s，候选 s_new 若 `conf > enter_hi` 持续 ≥ dwell → 切
2. `conf > hot_switch_floor` 跳过 dwell
3. 所有候选 `< exit_lo` → `Uncertain`（UI 显示 "thinking"，不 flap）

### 3.3 Property-based test
```rust
proptest! {
    fn no_flap_under_noise(seq in signal_near(0.65, 60s)) {
        let n = run_fsm(seq).transitions();
        prop_assert!(n <= 1);
    }
}
```
+ `test_hot_switch_overrides_dwell` / `test_exit_lo_never_backflap`

### 3.4 DoD
- [ ] Extractors 通过能量校准测试
- [ ] FSM `flaps_per_minute ≤ 1` 在 60s 合成噪声断言
- [ ] Hot switch 场景 < 500ms 切换
- [ ] Dwell time 被 property test 验证（1000 seed 固定）

---

## 4. M4 — Bench + Dataset v0 (9d, +Accuracy)

### 4.1 数据集规格
- **规模**: 5 scenes × 3 noise 等级 × 35 clips × 10s = **525 clips**
- **分割**: train 315 (60%) / dev 105 (20%) / held-out 105 (20%)
- **held-out 锁定**: release 前 sha256 锁，CI 只读

### 4.2 来源（经数据战略 Round D1 修正，STRATEGY §11.5）
- **开源**: AudioSet balanced subset + MUSAN + UrbanSound8K（筛选子集 ≤ 200 片段）
- **合成**: noise mixture（clean speech × noise at SNR -5/0/+10 dB）**限制: 合成样本占比 <30%，仅入 train，held-out 100% 真实录制**
- ~~**自采**: VoxSign dogfood~~ **不用**（VoxSign 数据仅作内部 rule-tuning，不入公开 pipeline；避免循环污染 + 身份隐私风险）

### 4.2.1 Speaker-disjoint split（QA 要求）
- 同一说话人声纹**不能跨** train/test
- Split 前构建 `speaker_registry.json`，split 后验证交集为空
- README 明示 speaker 总数（估 N ≥ 50，单 speaker 片段数 ≤ 15）

### 4.3 标注流程（$400 预算，Round 3 确认方案）
- 2 位 Prolific 专业标注员 × 525 片段核心层（$300-350）
- 作者 + 3 位 VoxSign 用户 dogfood 双标注 125 片段（免费）
- **kappa 门 ≥ 0.70**（0.75 过严 / 0.65 过松的折中）
- `scripts/validate_dataset.py` assert kappa，不过阻止 release
- kappa 不过 → D55 砍刀 L2（规模降到 150 片段）

### 4.4 托管 & 许可
- HuggingFace Datasets: `smithpeter/perceptkit-bench-v0`
- git-lfs 镜像: `dataset/` 目录
- 许可: 衍生片段 CC-BY-NC（研究用），metadata CC0

### 4.5 CI Gate（必过）
- **Accuracy gate**: held-out **macro-F1 ≥ 0.72** (主指标, 防长尾头部类主导) AND ECE ≤ 0.12 AND per-scene recall ≥ 0.70 AND Top-1 ≥ 0.78 (辅)
- **Speaker-disjoint gate**: speaker_registry 交集为空 (split 失败 → fail)
- **Synthetic ratio gate**: held-out synthetic 比例 = 0, train synthetic 比例 < 30%
- **Flapping gate**: 合成 60s 抖动 `flaps/min < 1`
- **Coverage gate**: held-out 命中率 ≥ 95%
- **Perf gate**: `evaluate_p95 < 5ms` on macOS arm64

### 4.6 DoD
- [ ] 525 片段全部标注完成，kappa ≥ 0.70（per-scene kappa 也报，任一 < 0.60 → fail）
- [ ] `speaker_registry.json` 发布 + speaker 数明示（估 ≥ 50）
- [ ] Synthetic 比例明示（held-out 0%, train <30%）
- [ ] HF dataset + **S3 mirror**（HF revision 非 immutable）+ sha 锁在 GitHub Release
- [ ] 6 个 CI gate 全绿（含新增 speaker-disjoint + synthetic ratio）
- [ ] README "Known Limitations" 写 v0.1 macro-F1 门 0.72 vs 战略 Top-1 0.85
- [ ] Datasheet 草稿（入 DATA.md, M7 完善）

---

## 5. M5 — PyO3 abi3 Binding (7d, +Performance)

### 5.1 Python API
```python
import numpy as np
import perceptkit as pk

engine = pk.SceneEngine.from_dir("./scenes")
decision = engine.analyze_audio(pcm_np, sample_rate=16000)
# SceneDecision(scene_id, confidence, description, source, rationale)

engine.on_transition(lambda prev, curr: print(f"{prev} → {curr}"))
```

### 5.2 零拷贝
- `PyArray1<f32>` C-contiguous 检查，非连续 → 显式 error（不自动 copy）
- dtype 严格 f32，非 f32 → `TypeError`

### 5.3 Typing
- `.pyi` stub 覆盖所有公共 API
- `mypy --strict` 绿

### 5.4 CI Wheel Matrix
- macOS arm64 + Linux x86_64 + Linux aarch64
- Python 3.11 / 3.12 / 3.13 (abi3)
- D60 砍刀 L3: 若超时，仅保留 macOS arm64 + Linux x86_64 + py311

### 5.5 DoD
- [ ] `pip install -e .` Mac/Linux 过
- [ ] pytest 覆盖 Python API ≥ 70%
- [ ] `mypy --strict` 绿
- [ ] 3 平台 wheel 构建通过

---

## 6. M6 — Reflector: Noop + Mock + Qwen-0.5B 本地 (13d, +Learnability +Adaptivity)

**Round 2 Red Team 指出 "Reflector 空壳 = Learnability 假"**，Round 3 决策 2-A 要求 v0.1 带真实本地 LLM。

### 6.1 三实现
- **`NoopReflector`**: 总是返回 `Unknown`。CI 默认。零依赖。
- **`MockReflector`**: VCR fixture 回放（pre-recorded prompt→response）。测试用。
- **`LocalReflector`**: 本地 llama.cpp + Qwen-0.5B Q4_K_M（复用 VoxSign 现有基础设施）
  - Feature flag: `--features local-reflector`
  - 模型下载: `perceptkit models download qwen-0.5b-q4`
  - 7 个 tool: `query_feature_history` / `list_known_scenes` / `compare_against_scene` / `describe_in_nl` / `propose_new_scene` / `request_more_signals` / `mark_unknown`

### 6.2 三出口
- **Map**: 返回已知 scene_id + rationale
- **Propose**: 生成 YAML 草稿 + 示例，进 `PendingSceneQueue`
- **Unknown**: FeatureSummary + NL 描述

### 6.3 Budget
```rust
pub struct ReflectionBudget {
    pub max_time_ms: u64,      // 默认 2000
    pub max_tokens: u32,       // 默认 1024
    pub max_tool_calls: u32,   // 默认 7
}
```
超限 → 强制 `Unknown`（永不 hang）

### 6.4 ReflectionTrace（JSONL 持久化）
```json
{"ts":..., "case_id":..., "input_features":{...}, "tool_calls":[...], "output":{...}, "model":"qwen-0.5b@v1", "prompt_hash":"..."}
```
`perceptkit replay <trace.jsonl>` 命令 100% 确定性重放。

### 6.5 CLI
- `perceptkit reflect <signals.json>` 单次触发 Reflector
- `perceptkit review list` 看 PendingQueue
- `perceptkit review approve <id>` 把 proposed YAML 写入 `scenes/`

### 6.6 测试策略
- **VCR fixture**: `tests/reflector/fixtures/*.jsonl` pre-recorded
- **Prompt snapshot**: `insta` crate 做 prompt 模板 golden test
- **Budget enforce**: `test_reflector_budget_time_ms` / `test_reflector_budget_tokens`
- **Schema validation**: proposed YAML 必须通过 `perceptkit lint`

### 6.7 DoD
- [ ] 三 Reflector 都编译 + 单测
- [ ] LocalReflector 在 macOS arm64 上 Qwen-0.5B Q4 跑通三出口
- [ ] VCR fixture 覆盖 Map / Propose / Unknown 各 5 个用例
- [ ] Budget enforce 被 test 覆盖
- [ ] `perceptkit review approve` 能把 LLM 提议变 `scenes/xxx.yaml`

---

## 7. M7 — VoxSign POC + DATA.md + Release (13d, 全维)

**注**: 从 12d 扩 13d 以容纳 DATA.md 三件套 + Signal 模型文档。VoxSign 数据不脱敏开源（D2），省下 -2d，加上 DATA.md +3d，净 +1d 吸收进原 2d 缓冲（见 §0 总览）。

### 7.1 VoxSign 内部 Case Study（非公开）
- 在 VoxSign 分支 `perceptkit-integration`
- 实现 `PerceptkitSceneAdapter` 作为 `SceneAnalyzer` 替代
- 翻译 5-10 个核心场景到 YAML（不追求全部 1741 行）
- VoxSign 真实数据仅做**内部 rule-tuning + baseline 评估**，不进 perceptkit-bench
- 公开的 case study 博客只讲"架构层面可替换"，不披露 VoxSign 用户数据细节

### 7.2 VoxSign 覆盖率审计（Round 3 决策）
- 脚本统计 VoxSign 1741 行 `edge/scene/` 在 v0.1 DSL 下表达率
- **门**: ≥ 40%
- 未达 → **silent release**（不发 HN，v0.2 再公开）
- README "Known Limitations" 诚实披露 temporal / stateful / per-user 缺口

### 7.3 DATA.md 三件套（新增，HELM 标准）
- **Datasheet v0**: 525 片段 完整描述（来源 / 标注流程 / 人口学 / 许可 / limitations）
- **Model Card v0**: v0.1 rule engine + Qwen-0.5B Reflector 能力 + 限制 + biases
- **Eval Card v0**: 评估指标定义 (macro-F1 / ECE / per-scene recall) + 复现命令 + baseline 对比
- **Signal 模型承诺**: `perceptkit` 二进制零 network call，`cargo-sbom` + egress audit 工具链
- **DCO 协议**: `CONTRIBUTING.md` + pre-commit hook 自动加 Signed-off-by

### 7.4 冷启动叙事（产品层）
- **Blog 1**: "Perception Benchmark v0" — 数据集 + datasheet + leaderboard，建立话语权
- **Blog 2**: "Why your LangChain agent is deaf" — 架构 demo（不依赖 VoxSign 真实数据）
- **HN post**: `Show HN: perceptkit — the perception layer LangChain forgot`（仅覆盖率 ≥40% 时发）

### 7.5 发布渠道
- **crates.io**: `perceptkit-core` + `perceptkit-audio`（`perceptkit-py` 不发）
- **PyPI**: `perceptkit` wheel (3 平台 × 3 Python 版本)
- **HuggingFace Datasets**: `smithpeter/perceptkit-bench-v0` 发布 + S3 mirror + sha 公证
- **GitHub Release**: tag `v0.1.0`，完整 CHANGELOG + 数据集 sha 锁定

### 7.6 DoD
- [ ] VoxSign POC 分支可跑通，替换 `edge/scene/classifier.py`
- [ ] 覆盖率审计报告 commit 到 `docs/voxsign-integration.md`（不含 VoxSign 用户数据）
- [ ] **DATA.md + Datasheet v0 + Model Card v0 + Eval Card v0 四件套完整**
- [ ] **DCO 协议 + CONTRIBUTING.md + pre-commit hook 就绪**
- [ ] **cargo-sbom 生成 + egress audit 证明 perceptkit 二进制零 network call**
- [ ] 2 篇博客写完（可发布状态）
- [ ] crates.io + PyPI + HuggingFace Datasets 三渠道发布
- [ ] GitHub Release v0.1.0 打 tag + 数据集 sha 公证

---

## 8. 质量门（CI 硬约束）

| 门 | 命令 | 阈值 |
|---|---|---|
| Accuracy | `cargo test --features bench -- accuracy_gate` | **macro-F1 ≥ 0.72** (主), Top-1 ≥ 0.78, ECE ≤ 0.12, per-scene recall ≥ 0.70 |
| Speaker-disjoint | `python scripts/validate_split.py` | speaker 交集 = ∅ |
| Synthetic ratio | `python scripts/validate_dataset.py` | held-out synthetic = 0%, train synthetic < 30% |
| Flapping | `cargo test -- flapping_gate` | `flaps/min < 1` |
| Scene lint | `perceptkit lint scenes/` | 0 未裁决冲突 + coverage ≥ 95% |
| Perf | `cargo bench -- perf_gate` | p95 < 5ms |
| Offline (Signal 模型) | `cargo test --no-default-features` + `cargo-sbom` + egress audit | core 零网络 + SBOM 无 `reqwest`/`tokio-net` |
| Python wheel | `maturin build --release` × 3 平台 | 全绿 |
| Type | `mypy --strict python/` | 0 error |
| LLM Reflector | `cargo test --features local-reflector -- reflector` | VCR + budget 全过 |
| DCO | `.github/workflows/dco.yml` | 所有 commit 含 `Signed-off-by:` |

---

## 9. Overrun Policy（超支应急）

**触发器 D45 / D55 / D60**（见 §0 总览）：

```
D45 未过 M4
  └─> 砍刀 L1: M7 HN/博客推迟到 v0.1.1
            v0.1 做 silent release, 仅 crates.io + PyPI + GitHub Release
            节省约 5d, 回落 60d 预期

D55 未过 M5 + M6 Noop
  └─> 砍刀 L2: 数据集 525 → 150 片段（标 v0.1 baseline）
            降低 kappa 门到 0.65
            节省约 4d, 回落 61d 预期

D60 未过 M6 Qwen 集成
  └─> 砍刀 L3: PyO3 abi3 → py311 专属
            CI 矩阵减半
            LocalReflector 延后 v0.2（保留 Noop + Mock）
            节省约 3-5d
```

**VoxSign 主线保护红线**: 任意时刻 perceptkit 累计占用作者 > 60d → 强制暂停 perceptkit 2 周恢复 VoxSign 主线。

---

## 10. 关键依赖（Cargo + Python）

### Rust
| Crate | 用途 | 说明 |
|---|---|---|
| `serde_yml` | YAML DSL | ⚠️ 不用 archived `serde_yaml` |
| `thiserror` | 错误 | core |
| `tracing` | 日志 | core |
| `ndarray` | 数值 | audio |
| `proptest` | property test | core (dev) |
| `insta` | snapshot test | core (dev) |
| `criterion` | benchmark | all (dev) |
| `pyo3` + `numpy` | Python binding | py |
| `async-trait` | Reflector trait | core |
| `rusqlite` | PendingQueue | core |
| `llama-cpp-2` | LocalReflector | 可选 feature `local-reflector` |

### Python
- `maturin` / `numpy` / `pytest` + `pytest-cov` / `ruff` / `mypy`

---

## 11. 非目标（v0.1 明确不做，见 STRATEGY.md §6 + §11）

- ASR / LLM 推理 in core / 音频采集 / streaming API / Temporal DSL / Stateful DSL / Per-user override / Vision / Text / 商业化 / Evolution Loop 自动 commit
- **Flywheel Telemetry** (永不做，见 STRATEGY §11.3 "Signal 模型"承诺)
- **CLA** (用 DCO，见 STRATEGY §11.7)
- **VoxSign 真实用户数据进 perceptkit 公开 pipeline** (见 STRATEGY §11.5)

---

## 12. 完成定义 (DoD, v0.1.0)

全部勾选后才能 tag `v0.1.0`：

- [ ] M1-M7 所有 DoD 满足
- [ ] CI 8 个质量门全绿
- [ ] `cargo test --workspace` 绿
- [ ] `pytest` 绿，覆盖率 ≥ 70%
- [ ] `cargo clippy -- -D warnings` 零警告
- [ ] crates.io 发布 `perceptkit-core` + `perceptkit-audio`
- [ ] PyPI 发布 `perceptkit` wheel
- [ ] README 含可运行 quickstart + Known Limitations
- [ ] 5 个内置场景 YAML + lint 无冲突
- [ ] `docs/voxsign-integration.md` + 覆盖率数据
- [ ] `docs/scene-dsl.md` / `docs/signal-protocol.md` / `docs/extending-providers.md`
- [ ] GitHub Release v0.1.0 tag

---

## 13. 与 STRATEGY.md 的关系

- **STRATEGY.md** 定战略：使命 / 五维权重 / Moat / 非目标 / Roadmap
- **plan.md**（本文件）定执行：milestone / DoD / 预算 / 砍刀
- 战略变更须红蓝军重新对抗；执行变更作者自主（但 DoD 不得弱化）
- 每月 retro: 基于 STRATEGY §2 五维打分，追踪走势

---

**此 plan 是 perceptkit v0.1 的工程契约。M1 脚手架开工前需用户签字"plan 确认，可以开 M1"。**
