"""perceptkit — AI Agent Context Layer.

Perception middleware for voice agents: turns multimodal signals into
declarative, auditable scene decisions.

See https://github.com/smithpeter/perceptkit for docs.
"""

from perceptkit._perceptkit import (
    __version__,
    audio_version,
    core_version,
    version,
)

__all__ = [
    "__version__",
    "version",
    "core_version",
    "audio_version",
]
