"""perceptkit — AI Agent Context Layer.

Perception middleware for voice agents: turns multimodal signals into
declarative, auditable scene decisions.

See https://github.com/smithpeter/perceptkit for docs.

Quickstart:
    >>> import numpy as np
    >>> import perceptkit as pk
    >>> engine = pk.SceneEngine.from_dir("./scenes")
    >>> decision = engine.analyze_audio(np.zeros(16000, dtype=np.float32), 16000)
    >>> print(decision.scene_id, decision.confidence)
"""

from perceptkit._perceptkit import (
    SceneDecision,
    SceneEngine,
    __version__,
    audio_version,
    core_version,
    version,
)

__all__ = [
    "SceneDecision",
    "SceneEngine",
    "__version__",
    "version",
    "core_version",
    "audio_version",
]
