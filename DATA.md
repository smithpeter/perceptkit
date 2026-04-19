# perceptkit — Data Governance & Documentation

> 数据治理权威文档。**v2.0（2026-04-19）经红蓝军 Round 5/6 数据飞轮元假设激活后重写**。配套 `STRATEGY.md §11+§12+§13` 定战略，本文件定执行细则。

---

## 1. 概览（v2.0 重写）

**用户元假设**："数据是终极护城河，算力/算法都能买到。"——这是项目元决策，本文件所有设计服从此。

perceptkit 数据战略 = **3 层 OSS 架构 + 1 个 vertical 旗舰**：

```
Layer 1 — Seed Dataset (v0.1 ✅)
  └─ 公开数据 + 合成 + 525 标注片段 → 建立可信度 baseline

Layer 2 — Community Contribution (v0.2)
  └─ `perceptkit contribute` CLI + DCO + CI 校验

Layer 3 — Data Flywheel (v0.2 新)  ⭐
  └─ perceptkit-cloud crate (opt-in) → telemetry / dataset upload
     core/audio/py 仍零网络（cloud 是独立模块）

Layer 4 — Vertical Data Asset (v0.2 新)  ⭐⭐
  └─ perceptkit-bench-knowledge-work-v0
     VoxSign 脱敏 + 合作方 + 公开数据 → 旗舰垂类壁垒
```

详见 `STRATEGY.md §11+§12+§13`。本文件聚焦执行细节。

---

## 2. Signal 模型 — v2.0 修订（部署级选择，非项目红线）

### 2.1 v1.0 → v2.0 变化

**v1.0 旧（已废弃）**：~~perceptkit 二进制永不有 network call~~
**v2.0 新**：根据**部署形态**决定，详见 `STRATEGY.md §12`。

| Crate | network 默认 | 工程保证 |
|---|---|---|
| `perceptkit-core` | **永远零网络** | `cargo deny` 持续封 reqwest/hyper/surf/ureq/awc |
| `perceptkit-audio` | **永远零网络** | 同上 |
| `perceptkit-py` | **永远零网络** | 同上 |
| **`perceptkit-cloud`**（v0.2 新）| opt-in，默认不编译 | 独立 crate + 独立 deny.toml + 独立 SBOM；用户必须显式 `cargo add perceptkit-cloud` 才存在 |

### 2.2 双承诺

**形态 A — Embedded mode（默认）**:
> "We can't collect your data because the binary doesn't have the ability to."

适用：医疗 / 法律 / 车载 / IoT / 隐私敏感开发者。承诺与 v1.0 相同，未弱化。

**形态 B — Cloud mode（opt-in）**:
> "You opt in. We tell you exactly what flows out, when, and where. You can stop anytime. We document every byte at `cloud-egress-spec.md`."

适用：数据飞轮参与者（VoxSign / 合作 OSS / 主动贡献者）。

### 2.3 工程隔离硬约束

- `perceptkit-cloud` 永远 depends on `perceptkit-core`，反向**永不**
- `perceptkit-core` 的 `cargo deny` 不放宽
- CI 矩阵保留双 profile：`default`（无 cloud）+ `with-cloud`
- `perceptkit-cloud` 任何上传必须 explicit user prompt + sanitization preview，绝不静默

### 2.4 如何贡献数据 — 两种路径（v2.0）

**路径 A — 完全手动 (沿用 v1.0)**:
```bash
perceptkit export --case <id> --out my_scene.yaml
$ cat my_scene.yaml      # 用户审核
git clone https://github.com/smithpeter/perceptkit-bench-v0
cp my_scene.yaml scenes/contrib/xxx.yaml
git commit -s
gh pr create
```

**路径 B — Cloud opt-in (v0.2 新)**:
```bash
# 1. 显式安装
cargo add perceptkit-cloud
# (Python: pip install perceptkit[cloud])

# 2. 显式启用并配置
perceptkit cloud init
# 交互 prompt:
#   - 你的贡献者邮箱?
#   - 上传哪些字段? (scene_id only / +features / +PCM hash)
#   - 上传频率? (每次/每天聚合/每周聚合)
#   - 接受数据使用许可 CC-BY-NC? (y/N)

# 3. 每次上传前 preview
perceptkit cloud preview      # 显示即将上传的内容（可读）
perceptkit cloud upload       # 显式触发，不自动

# 4. 撤销
perceptkit cloud opt-out      # 立即停止，已贡献保留
perceptkit cloud delete       # 请求删除已贡献（按贡献者邮箱）
```

**关键不变**：路径 B 的每一步上传仍是用户**明确决定**，即使是 cloud 模式也不会"后台静默上报"。差别在于路径 B 提供了 CLI 自动化辅助，路径 A 需要全手工。

---

## 3. Datasheet — perceptkit-bench-v0

遵循 Gebru et al. 2018 "Datasheets for Datasets" 标准。

### 3.1 动机

- **为什么创建**: v0.1 评估 perceptkit 规则引擎的感知准确率
- **谁创建**: smithpeter + 2 位 Prolific 专业标注员 + 3 位 VoxSign 用户 dogfood 志愿双标注
- **资助来源**: 无。$400 自费标注预算。

### 3.2 组成

- **规模**: 525 clips × 10s = 5,250s ≈ 1.46 hour
- **类别**: 5 scenes
  - `office_quiet`
  - `online_meeting`
  - `driving`
  - `outdoor_noisy`
  - `multi_speaker_chat`
- **噪声档位**: 3 级（clean / moderate / heavy）
- **Split**: train 315 (60%) / dev 105 (20%) / held-out 105 (20%)
- **语言**: 英文 + 中文 + 阿拉伯语（VoxSign 三语对齐，但 VoxSign 数据不入 pipeline）

### 3.3 来源

- **开源（≤ 200 片段）**: AudioSet balanced subset + MUSAN + UrbanSound8K（筛选合适场景的子集）
- **合成（< 30%，仅入 train, held-out 0%）**: Clean speech × noise at SNR -5/0/+10 dB
- **ZhTTS / 用户自采** 某些场景 narration: `office_quiet` 手工录制，不含 PII

### 3.4 采集过程

- 开源数据: 直接用 HuggingFace `datasets` 库加载 AudioSet / MUSAN / UrbanSound8K，筛选子集
- 合成数据: `scripts/synthesize.py` 用 clean speech × noise at SNR 范围合成
- 采集时段: 2026-04 ~ 2026-05（M4 9 天）

### 3.5 标注

- **标注员**: 2 位 Prolific 语音标注 pro（$9-12/hr 合规价）+ 作者 + 3 位 VoxSign 用户志愿双标注 125 片段
- **预算**: $400（Prolific 核心 + 作者/志愿者免费）
- **kappa 门**: Cohen's kappa ≥ 0.70（0.75 过严 / 0.65 过松的折中）
- **per-scene kappa**: 任一类 < 0.60 → 数据集冻结，不 release
- **仲裁**: kappa < 0.70 的片段由第三标注员决定

### 3.6 人口学（声纹分布，防 speaker leakage）

- 估计 speaker 总数 ≥ 50
- 单 speaker 片段上限 ≤ 15
- **Speaker-disjoint split**: 同一 speaker **不能跨** train/test；`speaker_registry.json` 公开
- 性别 / 年龄 / 口音分布（估）: 男 55% / 女 45%；20-60 岁；英美普通话阿拉伯语 3 大类

### 3.7 预处理

- 所有 clip 重采样到 16 kHz mono WAV
- 去头尾静音 (< 50ms)
- RMS normalize 到 -23 LUFS

### 3.8 用途

- ✅ **允许**: 评估 perceptkit 或其他场景识别系统
- ✅ **允许**: 非商业研究 (CC-BY-NC 4.0)
- ❌ **禁止**: 商业产品训练（请购买商用授权）
- ❌ **禁止**: 说话人识别 / 声纹采集

### 3.9 许可

- **主数据集**: CC-BY-NC 4.0
- **Quickstart 样本（100 片段）**: CC0
- **衍生合成片段**: CC-BY-NC
- **Metadata**: CC0

### 3.10 Limitations & Biases

- **样本量小**: 525 片段无法覆盖真实世界分布长尾
- **语言偏**: 以英文为主，中文次之，其他语言未覆盖
- **噪声合成**: AWGN/babble 合成不完全模拟真实非稳态噪声（如驾驶车内多普勒）
- **场景少**: 只 5 类，v0.2 扩至 15 类
- **Single-label**: v0.1 单场景输出，真实世界常多标签（driving + music + chat）

### 3.11 托管 & 版本化

- 主仓库: `smithpeter/perceptkit-bench-v0` (GitHub + git-lfs)
- HuggingFace: `smithpeter/perceptkit-bench-v0`
- **S3 mirror**: `s3://perceptkit-bench-mirror/v0/` （HF revision 非 immutable）
- **sha 公证**: 每个 release `dataset.sha256` 文件锁在 GitHub Release，**永不改动**
- 下一版: `perceptkit-bench-v0.1` / `perceptkit-bench-v1.0`，老版本永 frozen

---

## 4. Model Card — v0.1 Rule Engine + Qwen-0.5B Reflector

遵循 Mitchell et al. 2019 "Model Cards for Model Reporting" 标准。

### 4.1 模型细节

- **Hot Path**: perceptkit-core rule engine (YAML DSL + FeatureDescriptor typed matching + Flapping FSM)
- **Cold Path**: Qwen-0.5B Q4_K_M via llama.cpp（仅 `--features local-reflector`）
- **Version**: v0.1.0
- **Date**: 2026-05 (预计 M7 完成)
- **License**: 代码 MIT OR Apache-2.0; Qwen-0.5B 遵循 Tongyi Qianwen License (Alibaba)

### 4.2 预期用途

- ✅ AI agent 的场景感知中间件
- ✅ 离线 / 边缘部署（iOS / 车载 / IoT）
- ✅ 可审计的场景决策（rationale + evidence）
- ❌ 不用于说话人识别
- ❌ 不用于替代专业 ASR / VAD

### 4.3 性能指标（held-out 105 片段）

v0.1 门（2026-05 update）：
- **macro-F1 ≥ 0.72**
- Top-1 ≥ 0.78
- ECE ≤ 0.12
- per-scene recall ≥ 0.70
- Hot Path p95 < 5ms
- Cold Path (Qwen) p95 < 2s
- Flapping < 1/min on 60s synthetic noise

### 4.4 局限 & 偏差

- **长尾失效**: 未训练场景 → LocalReflector 三出口（Map / Propose / Unknown），准确率未 100%
- **语言偏**: 英文最准（AudioSet 比例最大），阿拉伯语未单独评估
- **噪声偏**: 合成 AWGN 准 > 真实非稳态噪声
- **边界 flapping**: 场景置信度 0.5-0.7 区间有 Uncertain 状态不输出

### 4.5 训练与评估数据

见 Datasheet §3.2-3.6。**VoxSign 真实用户数据不进训练/评估**（见 STRATEGY §11.5）。

### 4.6 伦理考虑

- **隐私**: Signal 模型承诺（§2）确保用户数据永不离开本地
- **可审计**: 所有 SceneDecision 带 rationale；Qwen 所有调用 ReflectionTrace JSONL 可回放
- **偏见审计**: per-scene kappa + per-scene recall 监控；kappa < 0.60 或 recall < 0.50 任一类触发重审

### 4.7 警告与 Caveats

- v0.1 门 macro-F1 0.72 是**第一版基线**，战略目标 0.85 见 v0.2
- 不保证生产环境的场景识别效果（请在自己数据上验证）

---

## 5. Eval Card — v0.1 Evaluation Protocol

遵循 HELM (Holistic Evaluation of Language Models) 启发的评估规范。

### 5.1 评估指标定义

| 指标 | 定义 | 阈值 |
|---|---|---|
| **macro-F1** (主) | F1 per class 的算术平均（防长尾头部主导） | ≥ 0.72 |
| **Top-1 accuracy** (辅) | 分类头部命中率 | ≥ 0.78 |
| **ECE** (Expected Calibration Error) | 置信度校准 | ≤ 0.12 |
| **per-scene recall** | 每类召回率 | ≥ 0.70（单类 ≥ 30 样本才评估） |
| **Flapping rate** | 60s 信号切换次数 | < 1/min |
| **Latency p95** | Hot Path | < 5ms |

### 5.2 复现命令

```bash
# 数据集下载 + sha 验证
make download-bench-v0
sha256sum -c dataset.sha256  # 必须过

# 跑评估
cargo test --features bench --release -- accuracy_gate
python scripts/evaluate.py --dataset bench-v0 --split held-out
```

### 5.3 基线对比

| Baseline | macro-F1 | Top-1 | Notes |
|---|---|---|---|
| Random | 0.20 | 0.20 | 5-class balanced |
| Always-majority | 0.10 | 0.35 | 取 train 中最多的类 |
| perceptkit v0.1 rule only | 0.65 (估) | 0.72 (估) | 无 Reflector |
| perceptkit v0.1 rule + Qwen | **0.72**+ | **0.78**+ | 完整 Dual-Process |
| GPT-4o (one-shot prompt baseline) | 0.78 (估) | 0.85 | 每次 $0.03 cost, 2s latency |

**目的**: 证明 perceptkit hot path 以 <5ms / $0 成本达到 GPT-4o 7 分差距，is the "right middleware" 设计。

### 5.4 更新协议

- **v0.1 永 frozen**: sha256 锁，永远可作 legacy baseline
- **v0.2 升级**: 扩至 15 类 + 5000+ 片段，旧指标保留并列报告
- **新 model / 新 rule** release 必须**双版本报告**（v0.1-frozen + v{latest}-full）

---

## 6. Contribution Protocol (DCO)

### 6.1 为什么 DCO 不是 CLA

Round D1 产品视角数据：

| 协议 | 首贡漏斗 | 维护成本 | 法律力度 |
|---|---|---|---|
| CLA | 降 35% | 高（需 sign 服务） | 强 |
| DCO | 降 ~5% | 零（git commit hook） | 中 |

对个人项目 DCO 足够。参考 Linux / GitLab / Kubernetes (最新也接受 DCO)。

### 6.2 DCO 声明（Developer Certificate of Origin v1.1）

每个 commit 必须含：

```
Signed-off-by: Your Name <your.email@example.com>
```

贡献者签署即声明：

- (a) 贡献代码是本人原创，有权提交
- (b) 基于 open source license 修改，保留原 license
- (c) 贡献内容的记录是公开的

### 6.3 工具链

- **Pre-commit hook** (仓库自带): 自动加 `Signed-off-by`
- **GitHub Action** (`.github/workflows/dco.yml`): 阻止未签 commit 合并
- **模板** (`.github/CONTRIBUTING.md`): 贡献者指南 + DCO 全文

### 6.4 Scene YAML 贡献 workflow

```bash
# 1. fork perceptkit-bench-v0
gh repo fork smithpeter/perceptkit-bench-v0

# 2. 用 perceptkit 导出本地 case
perceptkit export --case <id> --out scenes/contrib/my_scene.yaml

# 3. 添加音频（CC0 license 要求）
cp my_audio.wav audio/contrib/my_scene_001.wav
# 必须 CC0，贡献者确认

# 4. DCO 签名提交
git commit -s -m "add: my_scene contribution"

# 5. 提 PR
gh pr create --title "add my_scene (CC0)"
```

### 6.5 审核流程

| 贡献者 reputation | 审核比例 | SLA |
|---|---|---|
| 首次贡献 (rep 0) | **100% 人审** | 7 天 |
| 2-3 次已 merge (rep 1) | **50% 抽审** | 5 天 |
| 4+ 次无违规 (rep 2+) | **10% 抽审** | 3 天 |
| 任何违规发现 | **退回 rep 0 + 回到 100% 人审** | - |

审核内容：
1. YAML 通过 `perceptkit lint`（schema + 冲突检测）
2. 音频 CC0 许可确认
3. 人工听：音频内容与场景一致
4. 无 PII / 可识别声纹
5. 无对抗样本特征

### 6.6 Reputation Score

- `scripts/reputation.py` 计算每贡献者 rep
- 公开 leaderboard（可选，贡献者 opt-in 显示）

---

## 7. 数据集版本化规则

```
perceptkit-bench-v0.{minor}.{patch}
│  │
│  └─ minor: 类别数变化（5 → 15 扩展）
└─ patch: 仅加新片段/fix 标注错误, 不改类别

v0: 冻结
v0.1: +同类别新片段
v1.0: 类别升级 (15 类，新 schema)
v0 永远作 legacy baseline，每个 v{N+1} release 必须双报告
```

---

## 8. 待回答（v0.2 起研究）

- Temporal DSL 数据集（跨时间场景转换，如 meeting → driving）
- Multi-label 数据集（driving + chat + music 并存）
- Per-user personalization 数据集
- 真实世界 long-form audio（10 分钟以上）

---

## 9. perceptkit-bench-knowledge-work-v0 (v2.0 新增 — 旗舰 vertical)

> v2.0 数据飞轮的**主战场**。规模、来源、许可、治理与 §3 通用 bench-v0 并行但独立。

### 9.1 动机

- 用户元假设：数据 >> 算力/算法
- 3C vertical 决策：通用框架 + 1 个旗舰 vertical（知识工作者会议场景）建立数据壁垒
- 现状：真实知识工作会议数据被 Granola/Otter/Krisp 锁起来，公开 < 50h（ICSI ≈ 75h 但口音单一年代久）→ **真稀缺**

### 9.2 规模 & 来源（v0.2 → v0.3 阶梯）

| 维度 | v0.2 ship | v0.3 目标 |
|---|---|---|
| 总时长 | ≥ 200h labeled | ≥ 1000h |
| 来源 | VoxSign 脱敏 (60%) + 公开 ICSI/AMI (20%) + 合作方 (20%) | + 社区贡献 |
| 场景数 | 10 (meeting_zoom / huddle / interview / standup / brainstorm / 1on1 / pair_coding / focus_writing / async_recording / phone_call) | + 5 长尾场景 |
| 单类样本 | ≥ 30 | ≥ 100 |
| Speaker 数 | ≥ 200 | ≥ 1000 |

### 9.3 来源细则

**VoxSign 脱敏数据流（核心，2B 决策）**:
- 上游: VoxSign 用户 opt-in 协议（EULA 含产品改进数据共享条款）
- 处理: `voxsign-anonymizer` 工具链（VoxSign 仓内独立工具）
- 脱敏要求: PII 删除 + 声纹 hash + 时间戳粗化（仅保留小时级）+ 文本 GPT 改写（同义替换）
- 聚合粒度: 单 trace 不直传，最小聚合 = 100 同类场景的统计特征 + 5 抽样 sanitized clips
- 审核: 每批 VoxSign 法务 + 技术双签
- 撤销: 用户随时 opt-out，已贡献保留但停止后续

**公开数据**:
- ICSI Meeting Corpus (NIST, ≈ 75h)
- AMI Corpus (≈ 100h)
- Common Voice 选段 (≥ 3 speakers 子集)

**合作方数据（候选，需 Path Y' 探测）**:
- Granola / Superwhisper / Wispr Flow（probe 顺序见项目 memory）
- 合作模式：他们提供脱敏会议音频片段，换取贡献者 credit + 优先 vertical 场景定义参与权

### 9.4 标注规格

- **kappa 门**: ≥ 0.75（vertical 比通用容易达到）
- **per-scene kappa**: 任一类 < 0.65 → 数据集冻结，不 release
- **标注预算**: $1500（v0.2，比 v0.1 通用 bench 的 $400 高，因为时长 ≥ 200h vs 1.46h）
- **标注员**: 4 位 Prolific 专业标注（含会议体验 ≥ 1y） + 作者 + VoxSign 用户志愿
- **仲裁**: 三方独立标注后 majority vote，平局走 author 仲裁

### 9.5 许可

- **公开版本**: CC-BY-NC 4.0 (主仓库)
- **Quickstart 样本（30 片段，10 场景 × 3 sample）**: CC0
- **VoxSign 流入数据**: 仅入 internal 池 (`perceptkit-bench-internal`，私有)，公开样本必须人工挑选 + 二次脱敏 + 双签

### 9.6 治理

- **仓库**: `smithpeter/perceptkit-bench-knowledge-work-v0`（v0.2 阶段独立 repo）
- **托管**: HuggingFace Datasets 主 + S3 mirror + sha 公证
- **贡献协议**: DCO，新增 vertical-specific 字段（meeting platform / participant count / device type）
- **审核**: 同 §6.5 reputation tier，但首贡 100% 人审 + meeting domain expert review

### 9.7 红线（不允许发生）

- ❌ VoxSign 客户原始音频/文本/PII 进 public repo
- ❌ 单 trace 直传（最小聚合粒度 100）
- ❌ 数据被反向 codify 进 perceptkit 默认 scenes（保 vertical 通用性，不锁死服务 VoxSign）
- ❌ "数据流通" = "telemetry 自动上报"（必须 explicit opt-in + 用户可见，详见 §2.4 路径 B）
- ❌ Cross-vertical contamination（knowledge_work bench 不能掺 driving / outdoor）

### 9.8 评估指标

vertical 主指标 = **vertical macro-F1**（10 场景平均），不是通用 bench 的 5 类 macro-F1。

| 指标 | v0.2 门 | v0.3 目标 |
|---|---|---|
| vertical macro-F1 | ≥ 0.70 | ≥ 0.85 |
| meeting/non-meeting binary F1 | ≥ 0.92 | ≥ 0.96 |
| 单场景 recall (任一) | ≥ 0.55 | ≥ 0.75 |
| Cohen's kappa (rule vs human) | ≥ 0.55 | ≥ 0.75 |

vertical bench v0.2 的成功定义：**至少 1 个第三方研究/产品引用作为 evaluation baseline**。

---

## 9. 参考文献

- Gebru et al. 2018. "Datasheets for Datasets"
- Mitchell et al. 2019. "Model Cards for Model Reporting"
- Stanford HELM Benchmark
- Common Voice project (Mozilla)
- Apple Differential Privacy Overview
- Signal Protocol (whisper systems)
- DCO v1.1 (developer certificate of origin)

---

**本文件版本**: 1.0 (2026-04-18，经数据战略 Round D1 对抗定版)

**变更协议**: 任何修改须走 PR + 红蓝军对抗（如果影响 §2 Signal 模型、§3.9 许可、§6 DCO 协议）。
