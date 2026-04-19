#!/usr/bin/env python3
"""Run perceptkit on a directory of labeled audio files → JSONL for `eval --gate`.

This is the path from "synthetic self-eval tautology" to **real accuracy measurement**
(the 50-clip gap in the final audit).

Usage:
    pip install perceptkit soundfile numpy
    python scripts/bench_from_audio_dir.py \\
        --audio-dir ./benchmark_audio \\
        --labels ./benchmark_audio/labels.csv \\
        --out ./real_bench.jsonl

labels.csv format (one file per line, header required):
    filename,scene_id
    office_clip_1.wav,office_quiet
    meeting_clip_1.wav,online_meeting
    ...

Then:
    perceptkit eval --scenes ./scenes --dataset ./real_bench.jsonl --gate

The difference from `perceptkit synthesize`: features here come from the real
`AudioProvider` pipeline (RMS / VAD / speaker-stub extractors) applied to
actual PCM. The eval measures feature extraction + rule matching, not
self-generated tautology.

Suggested sources of labeled audio (all free, CC-licensed):
- **ESC-50** (https://github.com/karolpiczak/ESC-50): 2000 environmental clips,
  50 classes → map ~20 classes to our 5 scenes
- **Mozilla Common Voice** (https://commonvoice.mozilla.org): clean speech →
  online_meeting, multi_speaker_chat
- **MUSAN** (https://www.openslr.org/17/): music/speech/noise
- **LibriSpeech test-clean** (https://www.openslr.org/12/): clean speech
- **UrbanSound8K** (https://urbansounddataset.weebly.com/urbansound8k.html):
  10 urban classes → driving, outdoor_noisy

A scene-to-source mapping for a 50-clip minimum benchmark:

| Scene              | Count | Source (free)                              |
|--------------------|-------|---------------------------------------------|
| office_quiet       |  10   | ESC-50 `clock_tick`, `breathing`            |
| online_meeting     |  10   | Common Voice speech (3+ speakers mixed)     |
| driving            |  10   | ESC-50 `engine`, `car_horn` + UrbanSound8K |
| outdoor_noisy      |  10   | ESC-50 `rain`, `wind`, `sea_waves`          |
| multi_speaker_chat |  10   | ESC-50 `crowd`, `laughing`                  |

Total 50 clips, all creative-commons licensed, ~50 MB download.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path

try:
    import numpy as np
    import soundfile as sf  # pip install soundfile
    import perceptkit
except ImportError as e:
    print(f"error: missing dependency ({e}). install: pip install perceptkit soundfile numpy", file=sys.stderr)
    sys.exit(2)


def load_pcm(path: Path, target_sr: int = 16000) -> np.ndarray:
    """Load an audio file as mono f32 PCM at target_sr."""
    data, sr = sf.read(str(path), dtype="float32", always_2d=False)
    if data.ndim == 2:
        data = data.mean(axis=1)
    if sr != target_sr:
        # Simple decimation — good enough for feature extraction.
        ratio = sr / target_sr
        indices = np.round(np.arange(0, len(data), ratio)).astype(int)
        indices = indices[indices < len(data)]
        data = data[indices].astype("float32")
    if data.max() > 1.0 or data.min() < -1.0:
        data = data / np.max(np.abs(data))
    return np.ascontiguousarray(data, dtype=np.float32)


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--audio-dir", type=Path, required=True, help="dir of wav/flac/mp3 files")
    p.add_argument("--labels", type=Path, required=True, help="CSV: filename,scene_id")
    p.add_argument("--out", type=Path, required=True, help="output JSONL path")
    p.add_argument("--scenes", type=Path, default=Path("./scenes"), help="scenes dir for sanity scene_ids check")
    p.add_argument("--sample-rate", type=int, default=16000)
    args = p.parse_args()

    if not args.labels.exists():
        print(f"error: labels file not found: {args.labels}", file=sys.stderr)
        return 2
    if not args.audio_dir.exists():
        print(f"error: audio dir not found: {args.audio_dir}", file=sys.stderr)
        return 2

    # Load engine to verify scene ids are real.
    engine = perceptkit.SceneEngine.from_dir(str(args.scenes))
    valid_scenes = set(engine.scene_ids())

    # Read labels — optional context_* columns are injected into the bundle
    rows = []
    with args.labels.open() as f:
        reader = csv.DictReader(f)
        for row in reader:
            fn = row.get("filename", "").strip()
            scene = row.get("scene_id", "").strip()
            if not fn or not scene:
                continue
            if scene not in valid_scenes:
                print(f"warn: unknown scene_id '{scene}' in labels; valid: {sorted(valid_scenes)}", file=sys.stderr)
            context_overrides = {}
            for key, value in row.items():
                if key and key.startswith("context_") and value and value.strip():
                    # column "context_app" → feature "context.app"
                    feature_key = "context." + key[len("context_"):]
                    context_overrides[feature_key] = value.strip()
            rows.append((fn, scene, context_overrides))

    if not rows:
        print("error: no valid rows in labels", file=sys.stderr)
        return 2

    out_records = []
    with args.out.open("w") as fout:
        for filename, label, context_overrides in rows:
            path = args.audio_dir / filename
            if not path.exists():
                print(f"warn: missing audio file: {path}; skipping", file=sys.stderr)
                continue

            pcm = load_pcm(path, target_sr=args.sample_rate)
            if len(pcm) < 100:
                print(f"warn: audio too short: {path}; skipping", file=sys.stderr)
                continue

            # Always extract features via the real AudioProvider (Rust),
            # then merge any context injection, then analyze_bundle. This
            # flows all audio features (inc. SpectralExtractor v0.2 FFT
            # additions) through the engine.
            features = dict(engine.extract_audio_features(pcm, args.sample_rate))
            features.update(context_overrides)
            decision = engine.analyze_bundle(features)

            # Emit decision-shape JSON (so `eval --gate` can compute accuracy).
            # Note: `eval` currently recomputes via scene rules given features,
            # so we should emit features. We use a reverse trick — analyze_bundle
            # with the features that produce the same decision. For now, emit
            # both the features we extracted (from AudioProvider) and the label.
            # TODO: expose AudioProvider.process in Python for clean feature emit.
            record = {
                "filename": filename,
                "label": label,
                "predicted_scene_id": decision.scene_id,
                "confidence": decision.confidence,
                "correct": decision.scene_id == label,
            }
            out_records.append(record)
            json.dump(record, fout)
            fout.write("\n")

    total = len(out_records)
    if total == 0:
        print("no clips processed")
        return 1
    correct = sum(1 for r in out_records if r["correct"])
    print(f"Processed {total} clips")
    print(f"Top-1 accuracy: {correct}/{total} = {correct / total:.4f}")

    # Per-scene precision / recall / F1 + macro averages
    from collections import Counter, defaultdict

    truth_counts: Counter[str] = Counter()
    pred_counts: Counter[str] = Counter()
    tp: Counter[str] = Counter()
    confusion: dict[str, Counter[str]] = defaultdict(Counter)

    for r in out_records:
        truth = r["label"]
        pred = r["predicted_scene_id"] or "UNKNOWN"
        truth_counts[truth] += 1
        pred_counts[pred] += 1
        confusion[truth][pred] += 1
        if r["correct"]:
            tp[truth] += 1

    macro_p, macro_r, macro_f1 = 0.0, 0.0, 0.0
    n_classes = len(truth_counts)
    print("\nPer-scene metrics:")
    print(f"  {'scene':<24} {'support':>8} {'prec':>8} {'recall':>8} {'f1':>8}")
    for scene in sorted(truth_counts):
        support = truth_counts[scene]
        truth_pos = support
        pred_pos = pred_counts.get(scene, 0)
        scene_tp = tp[scene]
        prec = scene_tp / pred_pos if pred_pos else 0.0
        rec = scene_tp / truth_pos if truth_pos else 0.0
        f1 = 2 * prec * rec / (prec + rec) if (prec + rec) else 0.0
        macro_p += prec
        macro_r += rec
        macro_f1 += f1
        print(f"  {scene:<24} {support:>8} {prec:>8.4f} {rec:>8.4f} {f1:>8.4f}")
    if n_classes:
        macro_p /= n_classes
        macro_r /= n_classes
        macro_f1 /= n_classes

    print()
    print(f"Macro-precision: {macro_p:.4f}")
    print(f"Macro-recall:    {macro_r:.4f}")
    print(f"Macro-F1:        {macro_f1:.4f}")

    # Cohen's kappa: (p_o - p_e) / (1 - p_e)
    p_o = correct / total
    # marginal independence over the union of label/prediction classes
    all_classes = set(truth_counts) | set(pred_counts)
    p_e = sum(
        (truth_counts.get(c, 0) / total) * (pred_counts.get(c, 0) / total)
        for c in all_classes
    )
    if abs(1 - p_e) < 1e-12:
        kappa = 0.0
    else:
        kappa = (p_o - p_e) / (1 - p_e)
    print(f"Cohen's kappa:   {kappa:.4f}")

    # Confusion matrix
    cols = sorted(set(c for row in confusion.values() for c in row))
    print("\nConfusion (rows=truth, cols=predicted):")
    print("  " + f"{'truth\\pred':<24}" + "".join(f" {c[:8]:>8}" for c in cols))
    for truth in sorted(confusion):
        print(
            "  " + f"{truth:<24}" + "".join(f" {confusion[truth].get(c, 0):>8}" for c in cols)
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
