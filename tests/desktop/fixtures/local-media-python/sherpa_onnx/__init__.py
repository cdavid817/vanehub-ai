"""Test-only stand-in for `sherpa_onnx`.

Presents the configuration classes and `OfflineTts` that
`vane_local_media_worker.sherpa_onnx_tts_engine` constructs, and returns generated audio with the
`sample_rate` and `samples` attributes the production wrapper reads before writing the WAV itself.
The WAV writing, path admission, and result conversion all stay production code.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path
from typing import Any, List

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import _vanehub_scenario as scenario  # noqa: E402

__version__ = "1.10.0-vanehub-fixture"

ENGINE = "sherpa-onnx"

_SAMPLE_RATE = 22_050


class _Config:
    """The config classes only need to carry their keywords; the wrapper never reads them back."""

    def __init__(self, **kwargs: Any) -> None:
        self.kwargs = kwargs


class OfflineTtsVitsModelConfig(_Config):
    pass


class OfflineTtsKokoroModelConfig(_Config):
    pass


class OfflineTtsMatchaModelConfig(_Config):
    pass


class OfflineTtsModelConfig(_Config):
    pass


class OfflineTtsConfig(_Config):
    pass


class _GeneratedAudio:
    __slots__ = ("sample_rate", "samples")

    def __init__(self, sample_rate: int, samples: List[float]) -> None:
        self.sample_rate = sample_rate
        self.samples = samples


class OfflineTts:
    def __init__(self, config: Any) -> None:
        self.config = config

    def generate(self, text: str, sid: int = 0, speed: float = 1.0) -> _GeneratedAudio:
        entry = scenario.load(ENGINE)
        behaviour = scenario.apply_common(ENGINE, entry)

        if behaviour == scenario.ENGINE_ERROR:
            raise RuntimeError(scenario.failure_message(entry))
        if behaviour == scenario.MALFORMED_RESULT:
            # No sample rate: the production wrapper must reject this rather than write a WAV
            # header it cannot fill in.
            return _GeneratedAudio(0, [])
        if behaviour == scenario.EMPTY:
            return _GeneratedAudio(_SAMPLE_RATE, [])

        # Duration scales with the text so a spec can tell a long utterance from a short one, and
        # a tone rather than silence so the written WAV has non-trivial content.
        seconds = min(3.0, max(0.25, len(text) / 40.0)) / max(0.5, min(2.0, speed))
        count = int(_SAMPLE_RATE * seconds)
        return _GeneratedAudio(
            _SAMPLE_RATE,
            [0.25 * math.sin(2 * math.pi * 440 * index / _SAMPLE_RATE) for index in range(count)],
        )
