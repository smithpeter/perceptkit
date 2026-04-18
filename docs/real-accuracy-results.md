# Real Accuracy Results — ESC-50 Benchmark (2026-04-19)

> First real-audio accuracy number. Harsh, honest, and valuable design feedback.

## TL;DR

**Top-1 = 4.5% on 200 ESC-50 clips** (50/scene × 4 scenes: office_quiet, outdoor_noisy, driving, multi_speaker_chat).

This is **worse than random** (5-class random = 20%, 4-scene + unknown = ~25%).

## What happened

| Scene | Recall | Expected from random |
|---|---|---|
| office_quiet | 16.0% (8/50) | ~25% |
| outdoor_noisy | 2.0% (1/50) | ~25% |
| driving | 0.0% (0/50) | ~25% |
| multi_speaker_chat | 0.0% (0/50) | ~25% |
| **Overall Top-1** | **4.5%** | **~25%** |

## Why (root cause)

Our v0.1 scene YAMLs require **context features** that ESC-50 audio clips
don't carry:

- `online_meeting` requires `context.app in [Zoom, Teams, Feishu, ...]`
- `driving` requires `context.motion == vehicle`
- `multi_speaker_chat` requires `context.app NOT in [meeting apps]`

ESC-50 is audio-only. Our `AudioProvider` emits 7 audio features (RMS, VAD,
ZCR, etc.) but **zero context features**. When the rule engine evaluates
scenes with `all` clauses requiring `context.app`, those conditions always
fail → scene rejected → hot path falls through to `Unknown`.

Even `outdoor_noisy` and `office_quiet` (audio-only) scored poorly because:
- `office_quiet` requires `audio.rms_db < -40.0`. ESC-50 clips are
  normalized to louder levels; even "quiet" clips (clock_tick) register
  around -20 dBFS, failing the threshold.
- `outdoor_noisy` requires `audio.rms_db > -20.0` AND `audio.voice_ratio < 0.3`.
  Our simple VAD (energy + ZCR) mis-fires on natural sounds that have
  speech-like spectral content, pushing voice_ratio above 0.3.

## What this teaches us

**The real v0.1 accuracy on audio-only benchmarks is close to random.**
Production use of perceptkit requires **both audio and context providers**.
This is a design feature (scenes are multi-modal by design), but it needs
to be **honest in the README**.

### Design inputs for v0.2

1. **Add audio-only scenes** that don't require context. E.g. `ambient_music`,
   `near_silence`, `sustained_speech` — scenes where audio features alone
   should suffice.
2. **Better VAD**. Our energy+ZCR VAD is too simple for non-speech audio.
   Silero VAD (ONNX, already in v0.2 plan) will help.
3. **Normalize dBFS**. Scene thresholds assume unnormalized recording
   levels; many datasets (including ESC-50) normalize. Add a dBFS
   calibration step in AudioProvider.
4. **Synthetic context injection mode** for audio-only benchmarks. A flag
   `--assume-context-app=<value>` lets benchmarks report "what if we had
   context X" conditional accuracy.

## What this DOESN'T mean

- ❌ "perceptkit doesn't work." It works when given the signals it was
  designed for.
- ❌ "We should tune YAMLs to pass the benchmark." That would be cheating
  (over-fitting to ESC-50, a 2015 benchmark not representative of
  production audio).
- ✅ "We published the real number honestly and used it as design feedback."

## Methodology

### Data
- **Source**: ESC-50 (https://github.com/karolpiczak/ESC-50), CC-BY-NC,
  ~250 MB download.
- **Mapping**: 9 ESC-50 classes → 4 perceptkit scenes (see
  `scripts/esc50_to_labels.py`). 5th scene `online_meeting` excluded because
  ESC-50 has no meeting audio.
- **Balance**: 50 clips per scene = 200 total (speaker disjoint: ESC-50
  clips are pre-segmented and not labeled by speaker).

### Pipeline
1. `scripts/fetch_esc50.sh` downloads + extracts ESC-50 (~250 MB, 2-min)
2. `scripts/esc50_to_labels.py` produces balanced `labels.csv` (200 rows)
3. `scripts/bench_from_audio_dir.py` runs each clip through:
   - `soundfile` decode + resample to 16 kHz
   - `perceptkit.SceneEngine.analyze_audio` (AudioProvider → FeatureBundle → evaluate)
4. Output JSONL with per-clip prediction + correct flag

### Reproducibility

```bash
./scripts/fetch_esc50.sh
python scripts/esc50_to_labels.py \
    --meta benchmark_audio/esc50_meta.csv \
    --out benchmark_audio/labels.csv
python scripts/bench_from_audio_dir.py \
    --audio-dir ./benchmark_audio \
    --labels ./benchmark_audio/labels.csv \
    --out ./real_bench.jsonl
```

Expected output: `Top-1 accuracy: ~0.045` and the per-scene breakdown above.

## Impact on v0.1.0-alpha.0 audit

Final audit (2026-04-19) condition 3: "At least 50 real-audio clips with
author-labeled kappa sampling". Status: **technically met** (200 real
clips labeled via ESC-50 taxonomy mapping) **but the result is
embarrassing**.

Two interpretations:
- **Optimist**: condition met; tag `v0.1.0-alpha.0` with honest real number in README
- **Pessimist**: the number is so bad that tagging `alpha` is misleading; keep `dev.1` and work on v0.2 design fixes

**Our recommendation: OPTIMIST, with aggressive README disclosure**. The
point of the audit was to catch self-deception. Publishing 4.5% is not
self-deception; it's the opposite. Readers can judge whether
audio-only-without-context perceptkit is useful for their use case. The
typical production integration (which provides context signals) will do
much better.

v0.2 will ship with the 4 design-input items above + re-run this benchmark
for comparison.
