"""Opt-in smoke check against real, locally installed engines.

Never run by CI and never run by `npm test`. It exists so that a developer who has actually
installed PaddleOCR, faster-whisper, or sherpa-onnx can confirm the bridge drives them, and so that
the result can be recorded as evidence rather than asserted.

    python smoke.py --engine faster-whisper

Configuration comes from environment variables, one per profile field, because the alternative is a
command line long enough that nobody would run it twice. Every engine is skipped -- reported as
NOT RUN with the missing variable named -- unless its required variables are set. A skip is never
reported as a pass.

Two rules this script does not bend:

- It downloads nothing. `HF_HUB_OFFLINE` and `TRANSFORMERS_OFFLINE` are set before any engine is
  imported, exactly as the Rust host sets them, and every model path must already exist.
- It prints no content and no paths. Recognized text, transcripts, and synthesis input are the
  user's; what lands on the terminal is a version, a device, a duration, and a character count.
"""

from __future__ import annotations

import argparse
import os
import sys
import tempfile
import threading
import time
import wave
from typing import Any, Callable, Dict, List, Optional, Tuple

os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from vane_local_media_worker import errors, protocol  # noqa: E402

PASSED = "PASSED"
FAILED = "FAILED"
NOT_RUN = "NOT RUN"


def _env(name: str) -> Optional[str]:
    value = os.environ.get(name)
    return value if value else None


def _missing(names: List[str]) -> List[str]:
    return [name for name in names if not _env(name)]


def _report(engine: str, status: str, detail: str) -> None:
    print(f"{engine}: {status} - {detail}")


def _cancel() -> threading.Event:
    return threading.Event()


def _synthetic_speech_wav(path: str) -> None:
    """A one-second 440 Hz tone at 16 kHz mono.

    Deliberately not speech. The point of the STT check is that a real model loads, accepts a real
    WAV, and returns without raising; a tone reliably transcribes to nothing, and `NO_SPEECH_DETECTED`
    is a passing outcome here. Shipping a recording of a human voice as a fixture would put someone's
    speech in the repository for no additional signal.
    """

    import math
    import struct

    frames = bytearray()
    for index in range(16_000):
        sample = int(12_000 * math.sin(2 * math.pi * 440 * index / 16_000))
        frames += struct.pack("<h", sample)
    with wave.open(path, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(16_000)
        handle.writeframes(bytes(frames))


def check_ocr(root: str) -> Tuple[str, str]:
    required = ["VANEHUB_SMOKE_OCR_IMAGE"]
    has_paddlex = bool(_env("VANEHUB_SMOKE_OCR_PADDLEX_CONFIG"))
    if not has_paddlex:
        required += [
            "VANEHUB_SMOKE_OCR_DETECTION_DIR",
            "VANEHUB_SMOKE_OCR_RECOGNITION_DIR",
        ]
    missing = _missing(required)
    if missing:
        return NOT_RUN, f"set {', '.join(missing)}"

    from vane_local_media_worker import paddle_ocr_engine

    source = os.path.realpath(str(_env("VANEHUB_SMOKE_OCR_IMAGE")))
    # The worker confines media to the operation root, so the sample is copied in rather than read
    # from wherever the developer keeps it.
    staged = os.path.join(root, "sample" + os.path.splitext(source)[1])
    with open(source, "rb") as reader, open(staged, "wb") as writer:
        writer.write(reader.read())

    params: Dict[str, Any] = {
        "sourcePath": staged,
        "paddleXConfigPath": _env("VANEHUB_SMOKE_OCR_PADDLEX_CONFIG"),
        "textDetectionModelDir": _env("VANEHUB_SMOKE_OCR_DETECTION_DIR"),
        "textRecognitionModelDir": _env("VANEHUB_SMOKE_OCR_RECOGNITION_DIR"),
        "textLineOrientationModelDir": _env("VANEHUB_SMOKE_OCR_ORIENTATION_DIR"),
        "language": _env("VANEHUB_SMOKE_OCR_LANGUAGE") or "ch",
        "device": _env("VANEHUB_SMOKE_OCR_DEVICE") or "cpu",
        "maxPdfPages": 20,
        "maxOutputCharacters": 200_000,
    }

    probe = paddle_ocr_engine.probe(params)
    started = time.monotonic()
    result = paddle_ocr_engine.ocr(params, _cancel())
    elapsed = int((time.monotonic() - started) * 1000)

    characters = sum(len(page["text"]) for page in result["pages"])
    return PASSED, (
        f"paddleocr {probe['packageVersion']} device={probe['device']} "
        f"pages={len(result['pages'])} characters={characters} elapsedMs={elapsed}"
    )


def check_stt(root: str) -> Tuple[str, str]:
    missing = _missing(["VANEHUB_SMOKE_STT_MODEL_DIR"])
    if missing:
        return NOT_RUN, f"set {', '.join(missing)}"

    from vane_local_media_worker import faster_whisper_engine

    audio = _env("VANEHUB_SMOKE_STT_AUDIO")
    staged = os.path.join(root, "utterance.wav")
    if audio:
        with open(os.path.realpath(audio), "rb") as reader, open(staged, "wb") as writer:
            writer.write(reader.read())
    else:
        _synthetic_speech_wav(staged)

    params: Dict[str, Any] = {
        "audioPath": staged,
        "modelDirectory": _env("VANEHUB_SMOKE_STT_MODEL_DIR"),
        "device": _env("VANEHUB_SMOKE_STT_DEVICE") or "cpu",
        "computeType": _env("VANEHUB_SMOKE_STT_COMPUTE_TYPE") or "int8",
        "language": _env("VANEHUB_SMOKE_STT_LANGUAGE") or "auto",
        "vadFilter": True,
        "beamSize": 5,
    }

    probe = faster_whisper_engine.probe(params)
    started = time.monotonic()
    try:
        result = faster_whisper_engine.transcribe(params, _cancel())
    except errors.WorkerError as failure:
        if failure.code == errors.NO_SPEECH_DETECTED:
            # Expected for the synthetic tone: the model ran and found no speech, which is the
            # outcome the composer treats as information rather than an error.
            elapsed = int((time.monotonic() - started) * 1000)
            return PASSED, (
                f"faster-whisper {probe['packageVersion']} device={probe['device']} "
                f"outcome=NO_SPEECH_DETECTED elapsedMs={elapsed}"
            )
        raise
    elapsed = int((time.monotonic() - started) * 1000)
    return PASSED, (
        f"faster-whisper {probe['packageVersion']} device={probe['device']} "
        f"characters={len(result['text'])} elapsedMs={elapsed}"
    )


def check_tts(root: str) -> Tuple[str, str]:
    missing = _missing(["VANEHUB_SMOKE_TTS_MODEL", "VANEHUB_SMOKE_TTS_TOKENS"])
    if missing:
        return NOT_RUN, f"set {', '.join(missing)}"

    from vane_local_media_worker import sherpa_onnx_tts_engine

    output = os.path.join(root, "speech.wav")
    params: Dict[str, Any] = {
        "outputPath": output,
        "text": "VaneHub local media smoke check.",
        "modelKind": _env("VANEHUB_SMOKE_TTS_MODEL_KIND") or "vits",
        "modelPath": _env("VANEHUB_SMOKE_TTS_MODEL"),
        "tokensPath": _env("VANEHUB_SMOKE_TTS_TOKENS"),
        "lexiconPath": _env("VANEHUB_SMOKE_TTS_LEXICON"),
        "dataDir": _env("VANEHUB_SMOKE_TTS_DATA_DIR"),
        "dictDir": _env("VANEHUB_SMOKE_TTS_DICT_DIR"),
        "voicesPath": _env("VANEHUB_SMOKE_TTS_VOICES"),
        "vocoderPath": _env("VANEHUB_SMOKE_TTS_VOCODER"),
        "ruleFsts": [],
        "speakerId": 0,
        "speed": 1.0,
        "numThreads": 1,
        "device": _env("VANEHUB_SMOKE_TTS_DEVICE") or "cpu",
    }

    probe = sherpa_onnx_tts_engine.probe(params)
    started = time.monotonic()
    result = sherpa_onnx_tts_engine.synthesize(params, _cancel())
    elapsed = int((time.monotonic() - started) * 1000)
    os.unlink(output)

    return PASSED, (
        f"sherpa-onnx {probe['packageVersion']} kind={params['modelKind']} "
        f"sampleRate={result['sampleRate']} durationMs={result['durationMs']} elapsedMs={elapsed}"
    )


CHECKS: Dict[str, Callable[[str], Tuple[str, str]]] = {
    protocol.ENGINE_PADDLE_OCR: check_ocr,
    protocol.ENGINE_FASTER_WHISPER: check_stt,
    protocol.ENGINE_SHERPA_ONNX: check_tts,
}


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(prog="local-media-smoke")
    parser.add_argument(
        "--engine",
        action="append",
        choices=list(CHECKS),
        help="Repeatable. Defaults to all three.",
    )
    parsed = parser.parse_args(argv)
    engines = parsed.engine or list(CHECKS)

    statuses = []
    for engine in engines:
        with tempfile.TemporaryDirectory(prefix="vanehub-smoke-") as root:
            os.environ["VANEHUB_LOCAL_MEDIA_ROOT"] = root
            try:
                status, detail = CHECKS[engine](root)
            except errors.WorkerError as failure:
                status, detail = FAILED, f"{failure.code} {failure.details}"
            except Exception as exc:  # noqa: BLE001 - a smoke check reports, it does not raise
                # Class name only, for the same reason the worker discards the message: a loader
                # routinely puts the model path into str(exc).
                status, detail = FAILED, type(exc).__name__
        _report(engine, status, detail)
        statuses.append(status)

    # NOT RUN is not a failure, and it is not a pass either. The caller records it verbatim.
    return 1 if FAILED in statuses else 0


if __name__ == "__main__":
    sys.exit(main())
