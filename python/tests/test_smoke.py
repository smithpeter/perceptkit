"""M1 smoke test — verify the Rust extension loads and exposes versions."""

import perceptkit


def test_version_is_str() -> None:
    assert isinstance(perceptkit.version(), str)
    assert perceptkit.version().startswith("0.")


def test_core_version_is_str() -> None:
    assert isinstance(perceptkit.core_version(), str)


def test_audio_version_is_str() -> None:
    assert isinstance(perceptkit.audio_version(), str)


def test_dunder_version() -> None:
    assert perceptkit.__version__ == perceptkit.version()
