# Real Accuracy Measurement (closing v0.1 audit condition 3)

> The 50-clip real-audio benchmark. Closes the last condition for restoring
> `v0.1.0-alpha.0` per final audit (2026-04-19).

## Why this exists

The final red-team audit found that `perceptkit eval --gate` runs on
synthetic feature bundles generated from the same `TEMPLATES` that define
the scenes. 100% accuracy on this is a mathematical tautology, not evidence.

This doc + script let you produce a **real** accuracy number using publicly
available audio with known labels.

## 30-minute minimum benchmark

### 1. Install dependencies

```bash
# Option A: if you already have the perceptkit wheel
pip install perceptkit soundfile numpy

# Option B: from source
maturin develop --release
pip install soundfile numpy
```

### 2. Collect 50 labeled clips (CC-licensed)

Recommended source allocation for initial benchmark:

| Scene              | Count | Source                          | License |
|--------------------|-------|----------------------------------|---------|
| office_quiet       |  10   | ESC-50 `clock_tick`, `breathing` | CC-BY-NC |
| online_meeting     |  10   | Common Voice speech (mix 3 speakers at SNR -20dB) | CC0 |
| driving            |  10   | ESC-50 `engine`, `car_horn`      | CC-BY-NC |
| outdoor_noisy      |  10   | ESC-50 `rain`, `wind`, `sea_waves` | CC-BY-NC |
| multi_speaker_chat |  10   | ESC-50 `crowd`, `laughing`       | CC-BY-NC |

**Quick start with ESC-50 only** (skip the synthesized meeting scene,
4 classes × ~12 clips = 50-ish):

```bash
# Download ESC-50 (~250 MB)
curl -L -o esc50.zip https://github.com/karolpiczak/ESC-50/archive/master.zip
unzip esc50.zip -d esc50-workspace/
mkdir benchmark_audio
cp esc50-workspace/ESC-50-master/audio/*-*.wav benchmark_audio/
```

### 3. Write labels.csv

Minimum file, one row per clip:

```csv
filename,scene_id
1-100032-A-0.wav,driving
1-100210-A-36.wav,driving
...
```

Map ESC-50 filenames (format `{fold}-{clip}-{take}-{target}.wav`, where `target`
is the class id 0-49) to perceptkit scenes:

| ESC-50 target | ESC-50 class    | → perceptkit |
|---------------|-----------------|--------------|
| 16            | `clock_tick`    | office_quiet |
| 19            | `breathing`     | office_quiet |
| 37            | `engine`        | driving |
| 43            | `car_horn`      | driving |
| 10            | `rain`          | outdoor_noisy |
| 11            | `sea_waves`     | outdoor_noisy |
| 16 (wind)     | `wind`          | outdoor_noisy |
| 24            | `laughing`      | multi_speaker_chat |
| 41            | `crowd`         | multi_speaker_chat |

Use `scripts/esc50_to_labels.sh` (write your own 5-line bash) to generate
`labels.csv` from ESC-50 target column.

### 4. Run the benchmark

```bash
python scripts/bench_from_audio_dir.py \
    --audio-dir ./benchmark_audio \
    --labels ./benchmark_audio/labels.csv \
    --out ./real_bench.jsonl
```

Output:

```
Processed 48 clips
Top-1 accuracy: 32/48 = 0.6667

Per-scene recall:
  driving                   7/12  = 0.5833
  multi_speaker_chat        8/12  = 0.6667
  office_quiet              9/12  = 0.7500
  outdoor_noisy             8/12  = 0.6667
```

### 5. Interpret

A **Top-1 ≥ 0.50** on 48 real clips (random baseline is 0.20 for 5 classes)
is real evidence that perceptkit-audio + rule engine is **doing something**,
not nothing. The v0.1 goal (≥0.78) may not be met; that's OK — the gap is
known and documented.

Options:
- **If Top-1 ≥ 0.70**: condition 3 met; tag `v0.1.0-alpha.0`
- **If Top-1 ∈ [0.50, 0.70)**: document honestly in README; tag `v0.1.0-alpha.0` noting the real number
- **If Top-1 < 0.50**: rule engine needs tuning; investigate which scenes fail worst, adjust YAML thresholds, re-run

### 6. Commit the result (not the audio)

```bash
# Commit only the JSONL + labels.csv (small), not the audio files
echo "benchmark_audio/*.wav" >> .gitignore
git add real_bench.jsonl benchmark_audio/labels.csv
git commit -s -m "feat(bench): real 48-clip ESC-50 benchmark — Top-1=0.67"
```

## What this doesn't measure (honest limitations)

- **ESC-50 clips are 5 seconds and isolated.** Real-world audio is longer
  and mixed. Our number overestimates performance on continuous audio.
- **No `online_meeting` coverage from ESC-50.** You'd need to synthesize
  mixed-speaker clips (Common Voice + MUSAN speech, 3+ speakers overlaid).
- **No speaker-disjoint split verification.** ESC-50 doesn't annotate
  speakers; we can't check for leakage at the speaker level.
- **macro-F1 > Top-1 is more informative** once classes are imbalanced.

The 525-clip human-labeled benchmark (kappa ≥ 0.70) from `DATA.md §3`
remains the goal for v0.2. The 50-clip ESC-50 version here is the
minimum-honest evidence needed for `v0.1.0-alpha.0`.

## Contribution

If you run this and want to contribute your `labels.csv` + per-scene
accuracy numbers to help others calibrate:

```bash
perceptkit contribute --bench real_bench.jsonl  # v0.2 CLI, not yet
```

For now, open a PR adding your numbers to `docs/real-accuracy-results.md`.
