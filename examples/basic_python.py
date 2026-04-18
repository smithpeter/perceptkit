"""Basic Python example: load scenes, evaluate a feature bundle.

Run from repo root (after `maturin develop`):
    python examples/basic_python.py
"""

from pathlib import Path

import numpy as np

import perceptkit


def main() -> None:
    scenes_dir = Path(__file__).parent.parent / "scenes"
    engine = perceptkit.SceneEngine.from_dir(str(scenes_dir))
    print(f"Loaded {engine.scene_count()} scenes: {engine.scene_ids()}")
    print(f"Extractors: {engine.extractor_names()}")

    # From pre-computed features
    decision = engine.analyze_bundle(
        {
            "audio.voice_ratio": 0.72,
            "context.app": "Zoom",
            "audio.speaker_count": 3.0,
            "audio.rms_db": -25.0,
        }
    )
    print(f"\nFrom bundle (meeting features):")
    print(f"  scene:       {decision.scene_id}")
    print(f"  confidence:  {decision.confidence:.4f}")
    print(f"  description: {decision.description!r}")
    print(f"  source:      {decision.source}")
    print(f"  rationale ({len(decision.rationale)}):")
    for line in decision.rationale:
        print(f"    - {line}")

    # From raw PCM (silence → office_quiet expected)
    pcm = np.zeros(16000, dtype=np.float32)
    decision = engine.analyze_audio(pcm, 16000)
    print(f"\nFrom silent audio:")
    print(f"  scene:      {decision.scene_id}")
    print(f"  confidence: {decision.confidence:.4f}")


if __name__ == "__main__":
    main()
