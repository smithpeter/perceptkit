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


# ESC-50 class name → perceptkit scene_id (None = skip)
# Reference: https://github.com/karolpiczak/ESC-50#dataset
ESC50_TO_PERCEPTKIT = {
    # --- office_quiet: low voice + low energy ---
    "clock_tick": "office_quiet",
    "breathing": "office_quiet",
    "clock_alarm": "office_quiet",  # sparse events in quiet room
    # --- driving: engine noise / road ---
    "engine": "driving",
    "car_horn": "driving",
    "siren": "driving",
    "train": "driving",  # transportation proxy
    # --- outdoor_noisy: nature / weather ---
    "rain": "outdoor_noisy",
    "sea_waves": "outdoor_noisy",
    "wind": "outdoor_noisy",
    "thunderstorm": "outdoor_noisy",
    "pouring_water": "outdoor_noisy",
    # --- multi_speaker_chat: multi-voice / crowd ---
    "crowd": "multi_speaker_chat",
    "laughing": "multi_speaker_chat",
    "clapping": "multi_speaker_chat",
    # --- online_meeting: single-speaker speech (we approximate) ---
    # ESC-50 has no true meeting audio. We use isolated human voice as a
    # proxy — acknowledging in real-accuracy.md this is imperfect.
    # (Nothing in ESC-50 cleanly fits online_meeting; skip.)
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
    with args.meta.open() as f:
        reader = csv.DictReader(f)
        for row in reader:
            esc_class = row.get("category", "").strip()
            scene = ESC50_TO_PERCEPTKIT.get(esc_class)
            if not scene:
                continue
            rows.append((row["filename"], scene, esc_class))
            scene_counts[scene] += 1

    with args.out.open("w") as f:
        w = csv.writer(f)
        w.writerow(["filename", "scene_id", "source_class"])
        for fn, scene, src in rows:
            w.writerow([fn, scene, src])

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
