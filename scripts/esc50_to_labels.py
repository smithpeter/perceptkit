#!/usr/bin/env python3
"""Generate labels.csv for perceptkit eval from ESC-50 meta CSV.

Maps ESC-50 classes (50 target categories) to the 5 perceptkit v0.1 scenes
with at least 10 clips per scene where possible.

Usage:
    python scripts/esc50_to_labels.py \\
        --meta benchmark_audio/esc50_meta.csv \\
        --out benchmark_audio/labels.csv
"""

from __future__ import annotations

import argparse
import csv
from collections import Counter
from pathlib import Path


# ESC-50 class name → (perceptkit scene_id, context overrides)
# Context injection simulates what a real multi-modal caller would provide
# (app / motion / window signals). See v0.2-design-inputs.md P3.
# Reference: https://github.com/karolpiczak/ESC-50#dataset
ESC50_TO_PERCEPTKIT: dict[str, tuple[str, dict[str, str]]] = {
    # --- near_silence: v0.2 audio-only scene (no context needed) ---
    "clock_tick": ("near_silence", {}),
    "breathing": ("near_silence", {}),
    "clock_alarm": ("near_silence", {}),
    # --- driving: needs context.motion=vehicle (engine-only class) ---
    "engine": ("driving", {"context_motion": "vehicle"}),
    "car_horn": ("driving", {"context_motion": "vehicle"}),
    "siren": ("driving", {"context_motion": "vehicle"}),
    "train": ("driving", {"context_motion": "vehicle"}),
    # --- outdoor_noisy: audio-only, no context needed ---
    "rain": ("outdoor_noisy", {}),
    "sea_waves": ("outdoor_noisy", {}),
    "wind": ("outdoor_noisy", {}),
    "thunderstorm": ("outdoor_noisy", {}),
    "pouring_water": ("outdoor_noisy", {}),
    # --- multi_speaker_chat: needs context.app = Messages (not meeting app) ---
    "crowd": ("multi_speaker_chat", {"context_app": "Messages"}),
    "laughing": ("multi_speaker_chat", {"context_app": "Messages"}),
    "clapping": ("multi_speaker_chat", {"context_app": "Messages"}),
}


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--meta", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args()

    if not args.meta.exists():
        print(f"error: meta file not found: {args.meta}")
        return 2

    scene_counts: Counter[str] = Counter()
    rows = []
    context_keys: set[str] = set()
    with args.meta.open() as f:
        reader = csv.DictReader(f)
        for row in reader:
            esc_class = row.get("category", "").strip()
            mapping = ESC50_TO_PERCEPTKIT.get(esc_class)
            if not mapping:
                continue
            scene, ctx = mapping
            context_keys.update(ctx.keys())
            rows.append((row["filename"], scene, esc_class, ctx))
            scene_counts[scene] += 1

    sorted_ctx_keys = sorted(context_keys)
    with args.out.open("w") as f:
        w = csv.writer(f)
        w.writerow(["filename", "scene_id", "source_class"] + sorted_ctx_keys)
        for fn, scene, src, ctx in rows:
            w.writerow([fn, scene, src] + [ctx.get(k, "") for k in sorted_ctx_keys])

    total = len(rows)
    print(f"Wrote {total} labels → {args.out}")
    print("Per scene:")
    for scene, count in scene_counts.most_common():
        print(f"  {scene:<24} {count:>3}")

    missing = {"office_quiet", "driving", "outdoor_noisy", "multi_speaker_chat"} - set(scene_counts)
    if missing:
        print(f"warn: no clips for: {missing}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
