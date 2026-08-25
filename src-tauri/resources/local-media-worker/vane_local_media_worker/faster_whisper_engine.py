"""faster-whisper bridge for whole-utterance transcription.

``local_files_only=True`` is not a preference here: without it, a model directory that fails to load
makes CTranslate2 fall through to a Hugging Face lookup, which turns a configuration mistake into a
silent network download.
"""

from __future__ import annotations

import threading
from typing import Any, Dict, Optional, Tuple

from . import errors, privacy

ENGINE = "faster-whisper"
CAPABILITIES = ["probe", "transcribe", "cancel", "shutdown"]

_VALID_COMPUTE_TYPES = frozenset({"auto", "int8", "float16", "int8_float16", "float32", "int8_float32"})
_model_cache: Dict[str, Any] = {}
_model_lock = threading.Lock()


def package_version() -> Optional[str]:
    try:
        import faster_whisper  # noqa: PLC0415

        version = getattr(faster_whisper, "__version__", None)
        return str(version) if version else None
    except Exception:  # noqa: BLE001
        return None


def _resolve_device(device: str) -> str:
    if device in {"cpu", "cuda", "auto"}:
        return device
    raise errors.WorkerError(errors.DEVICE_CONFIGURATION_INVALID, engine=ENGINE, device=device)


def _resolve_compute_type(compute_type: str) -> str:
    if compute_type not in _VALID_COMPUTE_TYPES:
        raise errors.WorkerError(
            errors.DEVICE_CONFIGURATION_INVALID, engine=ENGINE, field="computeType"
        )
    return compute_type


def _cache_key(model_directory: str, device: str, compute_type: str) -> str:
    return f"{model_directory}|{device}|{compute_type}"


def _model_for(params: Dict[str, Any]) -> Tuple[Any, str]:
    model_directory = privacy.admitted_model_path(
        params.get("modelDirectory"),
        engine=ENGINE,
        field="modelDirectory",
        required=True,
        expect_directory=True,
    )
    assert model_directory is not None  # noqa: S101 - required=True guarantees it
    device = _resolve_device(str(params.get("device", "auto")))
    compute_type = _resolve_compute_type(str(params.get("computeType", "auto")))
    key = _cache_key(model_directory, device, compute_type)

    with _model_lock:
        cached = _model_cache.get(key)
    if cached is not None:
        return cached, device

    try:
        from faster_whisper import WhisperModel  # noqa: PLC0415
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc

    try:
        model = WhisperModel(
            model_size_or_path=model_directory,
            device=device,
            compute_type=compute_type,
            local_files_only=True,
        )
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc

    with _model_lock:
        _model_cache[key] = model
    return model, device


def probe(params: Dict[str, Any]) -> Dict[str, Any]:
    version = package_version()
    if version is None:
        raise errors.WorkerError(errors.ENGINE_IMPORT_FAILED, engine=ENGINE)
    _, device = _model_for(params)
    return {
        "engine": ENGINE,
        "packageVersion": version,
        "device": device,
        "modelIdentity": "local-directory",
        "ready": True,
    }


def transcribe(params: Dict[str, Any], cancel: threading.Event) -> Dict[str, Any]:
    audio_path = privacy.admitted_media_path(
        params.get("audioPath"), engine=ENGINE, must_exist=True
    )
    model, device = _model_for(params)
    if cancel.is_set():
        raise errors.WorkerError(errors.OPERATION_CANCELLED, engine=ENGINE)

    language = params.get("language")
    resolved_language = None if language in (None, "", "auto") else str(language)
    beam_size = int(params.get("beamSize") or 5)
    vad_filter = bool(params.get("vadFilter", True))

    try:
        segments, info = model.transcribe(
            audio_path,
            language=resolved_language,
            vad_filter=vad_filter,
            beam_size=beam_size,
            word_timestamps=False,
        )
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc

    # faster-whisper returns a lazy generator; decoding errors surface only while draining it, so
    # the generator is exhausted here rather than after the host has been told the call succeeded.
    parts = []
    try:
        for segment in segments:
            if cancel.is_set():
                raise errors.WorkerError(errors.OPERATION_CANCELLED, engine=ENGINE)
            text = getattr(segment, "text", None)
            if text:
                parts.append(str(text))
    except errors.WorkerError:
        raise
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc

    transcript = "".join(parts).replace("\r\n", "\n").replace("\r", "\n").replace("\x00", "").strip()

    detected_language = getattr(info, "language", None)
    language_probability = getattr(info, "language_probability", None)
    duration = getattr(info, "duration", None)

    return {
        "text": transcript,
        "noSpeechDetected": not transcript,
        "detectedLanguage": str(detected_language) if detected_language else None,
        "languageProbability": (
            float(language_probability) if isinstance(language_probability, (int, float)) else None
        ),
        "durationMs": int(float(duration) * 1000) if isinstance(duration, (int, float)) else None,
        "device": device,
        "engineVersion": package_version(),
    }


def shutdown() -> None:
    with _model_lock:
        _model_cache.clear()
