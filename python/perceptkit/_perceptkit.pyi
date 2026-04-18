"""Type stubs for the Rust extension module."""

from typing import Any

__version__: str

def version() -> str: ...
def core_version() -> str: ...
def audio_version() -> str: ...

class SceneDecision:
    """Immutable scene decision produced by SceneEngine."""

    scene_id: str | None
    confidence: float
    description: str | None
    source: str
    rationale: list[str]

    def is_known(self) -> bool: ...
    def __repr__(self) -> str: ...


class SceneEngine:
    """Scene inference engine. Construct with `from_dir`."""

    @staticmethod
    def from_dir(path: str) -> "SceneEngine":
        """Load scene YAMLs from a directory."""
        ...

    def analyze_audio(
        self,
        pcm: Any,  # numpy.ndarray[np.float32]
        sample_rate: int,
    ) -> SceneDecision:
        """Run the full Audio → FeatureBundle → SceneDecision pipeline."""
        ...

    def analyze_bundle(self, features: dict[str, Any]) -> SceneDecision:
        """Analyze a pre-computed feature dict. Values must be bool / float / str."""
        ...

    def scene_ids(self) -> list[str]: ...

    def scene_count(self) -> int: ...

    def extractor_names(self) -> list[str]: ...

    @staticmethod
    def lint(path: str) -> dict[str, Any]:
        """Lint a scenes directory; returns {scenes_ok, conflicts, warnings, passed}."""
        ...
