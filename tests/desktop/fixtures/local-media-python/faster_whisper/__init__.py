"""Test-only stand-in for `faster_whisper`.

Presents what `vane_local_media_worker.faster_whisper_engine` uses: a module version, a
`WhisperModel` constructor with the production keyword set, and a `transcribe` returning a lazy
segment generator plus an info object. The generator is deliberately lazy, because the production
wrapper exhausts it inside the worker precisely so that a decode error surfaces before success is
reported -- a fixture returning a list would make that guarantee untestable.
"""

from __future__ import annotations

import sys
import wave
from pathlib import Path
from typing import Any, Dict, Iterator, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import _vanehub_scenario as scenario  # noqa: E402

__version__ = "1.0.0-vanehub-fixture"

ENGINE = "faster-whisper"

DEFAULT_TRANSCRIPT = "vanehub fixture transcript"


class _Segment:
    __slots__ = ("text",)

    def __init__(self, text: str) -> None:
        self.text = text


class _Info:
    __slots__ = ("language", "language_probability", "duration")

    def __init__(self, duration: float) -> None:
        self.language = "en"
        self.language_probability = 0.99
        self.duration = duration


class WhisperModel:
    def __init__(self, **kwargs: Any) -> None:
        # `local_files_only=True` is what stops CTranslate2 falling through to a hub lookup when a
        # model directory fails to load. If the production wrapper ever stopped passing it, this
        # fixture is the last place that would notice.
        if kwargs.get("local_files_only") is not True:
            raise ValueError("local_files_only must be True")
        if not Path(str(kwargs.get("model_size_or_path"))).is_dir():
            raise FileNotFoundError(kwargs.get("model_size_or_path"))
        self.kwargs = kwargs

    def transcribe(self, audio_path: str, **_kwargs: Any) -> Tuple[Iterator[_Segment], _Info]:
        entry = scenario.load(ENGINE)
        behaviour = scenario.apply_common(ENGINE, entry)

        if behaviour == scenario.ENGINE_ERROR:
            raise RuntimeError(scenario.failure_message(entry))

        path = Path(audio_path)
        if not path.is_file():
            raise FileNotFoundError(audio_path)
        # Reading the WAV is the point of the STT path: the host really did capture, encode, and
        # hand over a file, and a fixture that ignored it would let a broken writer pass.
        with wave.open(str(path), "rb") as handle:
            frames = handle.getnframes()
            rate = handle.getframerate() or 16_000
            if handle.getsampwidth() != 2 or handle.getnchannels() != 1:
                raise ValueError("expected 16-bit mono PCM from the host capture path")
        duration = frames / float(rate)

        text = "" if behaviour == scenario.EMPTY else str(entry.get("text") or DEFAULT_TRANSCRIPT)

        def segments() -> Iterator[_Segment]:
            if behaviour == scenario.MALFORMED_RESULT:
                # Fails while the production wrapper drains the generator, which is the case its
                # exhaustion guard exists for.
                raise RuntimeError("fixture decode failure while draining segments")
            if text:
                yield _Segment(text)

        return segments(), _Info(duration)


def _unused() -> Dict[str, Any]:
    return {}
