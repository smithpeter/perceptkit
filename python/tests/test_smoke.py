"""M5 Python API smoke test — verify full engine pipeline."""

from pathlib import Path

import numpy as np
import pytest

import perceptkit

SCENES_DIR = Path(__file__).parent.parent.parent / "scenes"


def test_version_is_str() -> None:
    assert isinstance(perceptkit.version(), str)
    assert perceptkit.version().startswith("0.")


def test_core_version_is_str() -> None:
    assert isinstance(perceptkit.core_version(), str)


def test_audio_version_is_str() -> None:
    assert isinstance(perceptkit.audio_version(), str)


def test_dunder_version() -> None:
    assert perceptkit.__version__ == perceptkit.version()


@pytest.fixture
def engine() -> perceptkit.SceneEngine:
    return perceptkit.SceneEngine.from_dir(str(SCENES_DIR))


def test_engine_loads_scenes(engine: perceptkit.SceneEngine) -> None:
    ids = engine.scene_ids()
    assert len(ids) >= 5
    assert "online_meeting" in ids


def test_engine_has_default_extractors(engine: perceptkit.SceneEngine) -> None:
    names = engine.extractor_names()
    assert any("EnergyExtractor" in n for n in names)
    assert any("VoiceActivityExtractor" in n for n in names)


def test_analyze_audio_silent_returns_known_or_unknown(
    engine: perceptkit.SceneEngine,
) -> None:
    """Silent audio should not crash — returns either Unknown or a matching scene."""
    pcm = np.zeros(16000, dtype=np.float32)
    decision = engine.analyze_audio(pcm, 16000)
    assert isinstance(decision, perceptkit.SceneDecision)
    # Silent audio has rms_db around -120, voice_ratio=0 — matches office_quiet
    # (rms_db < -40 AND voice_ratio < 0.2).
    assert decision.is_known()
    assert decision.scene_id == "office_quiet"
    assert 0.0 <= decision.confidence <= 1.0


def test_analyze_bundle_with_meeting_features(engine: perceptkit.SceneEngine) -> None:
    decision = engine.analyze_bundle(
        {
            "audio.voice_ratio": 0.72,
            "audio.rms_db": -25.0,
            "context.app": "Zoom",
            "audio.speaker_count": 3.0,
        }
    )
    assert decision.scene_id == "online_meeting"


def test_scene_decision_repr(engine: perceptkit.SceneEngine) -> None:
    pcm = np.zeros(16000, dtype=np.float32)
    decision = engine.analyze_audio(pcm, 16000)
    rep = repr(decision)
    assert "SceneDecision" in rep


def test_lint_on_starter_scenes() -> None:
    report = perceptkit.SceneEngine.lint(str(SCENES_DIR))
    assert report["scenes_ok"] >= 5
    # Starter scenes should pass lint (no unresolved conflicts)
    assert report["passed"]


def test_analyze_bundle_rejects_invalid_value_types(
    engine: perceptkit.SceneEngine,
) -> None:
    with pytest.raises(ValueError):
        engine.analyze_bundle({"audio.voice_ratio": [1.0, 2.0]})


def test_analyze_bundle_rejects_invalid_feature_key(
    engine: perceptkit.SceneEngine,
) -> None:
    with pytest.raises(ValueError):
        engine.analyze_bundle({"has space": 1.0})
