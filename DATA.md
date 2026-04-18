# perceptkit — Data Governance & Documentation

> 数据治理权威文档。经红蓝军 Round D1 对抗（4 视角：架构+隐私/产品+社区/商业+投资/QA+数据科学）确认。任何变更须重新对抗。
>
> **配套**: `STRATEGY.md §11` 定战略，本文件定执行细则。

---

## 1. 概览

perceptkit 的数据战略**不是堆数据**，是**建立可信度**。路径 D（不融资、不商业化）下，数据飞轮 ROI 为负，因此本项目采用**两层架构 + Signal 模型承诺**：

```
Layer 1 — Seed Dataset (v0.1)     ← 525 片段, CC-BY-NC, HF 托管
Layer 2 — Community Contribution (v0.2)  ← CLI + DCO + CI 校验
Layer 3 — [明确砍掉]                ← 永不编译 telemetry crate
```

---

## 2. Signal 模型承诺（最硬核）

**perceptkit 二进制零 network call**。

### 2.1 工程保证

- `perceptkit-core` 零网络依赖：`cargo deny` 禁 `reqwest` / `tokio-net` / `hyper` / `surf` / `ureq` / `awc`
- 所有 release `cargo-sbom` 证明
- GitHub Release 附 `egress_audit.txt`（`strace -e trace=network` 结果）
- 任何涉及网络的可选 feature（如 `local-reflector` 下载模型）**默认不编译**，需显式 `--features`

### 2.2 用户承诺

> "We can't collect your data because we don't have the ability to."

对比 Apple Differential Privacy（需调 ε 参数 + 20 人团队）和 Mozilla Common Voice（志愿上传）：
- DP 对个人项目不现实（Carlini 2020/2023 证 embedding inversion）
- Common Voice 要 Mozilla Foundation 支撑

perceptkit 选**更严格的 Signal 模型**（参考 Signal 协议"我们看不到"）。

### 2.3 如何贡献数据（opt-in，完全手动）

```bash
# 用户导出自己的 pending case（本地 SQLite）
perceptkit export --case <id> --out my_scene.yaml

# 用户审核 YAML + 音频（人眼可读）
$ cat my_scene.yaml

# 用户主动 PR
git clone https://github.com/smithpeter/perceptkit-bench-v0
cp my_scene.yaml scenes/contrib/xxx.yaml
git commit -s  # DCO signed
gh pr create
```

**关键**：perceptkit 自身**不会** upload，用户每一步**明确操作**。

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
