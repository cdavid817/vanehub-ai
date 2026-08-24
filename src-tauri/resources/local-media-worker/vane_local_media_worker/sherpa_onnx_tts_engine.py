"""sherpa-onnx offline TTS bridge.

Each model kind needs a different set of local files, so validation is per-kind rather than a single
permissive path list -- accepting "a model directory" would let a kokoro voice file stand in for a
vits lexicon and fail deep inside the native library with an unmappable error.
"""

from __future__ import annotations

import array
import threading
import wave
from typing import Any, Dict, List, Optional

from . import errors, privacy

ENGINE = "sherpa-onnx"
CAPABILITIES = ["probe", "synthesize", "cancel", "shutdown"]

MODEL_KINDS = ("vits", "piper", "kokoro", "matcha")
MAX_TEXT_CODE_POINTS = 4000

_engine_cache: Dict[str, Any] = {}
_engine_lock = threading.Lock()


def package_version() -> Optional[str]:
    try:
        import sherpa_onnx  # noqa: PLC0415

        version = getattr(sherpa_onnx, "__version__", None)
        return str(version) if version else "unknown"
    except Exception:  # noqa: BLE001
        return None


def _model_path(params: Dict[str, Any], field: str, *, required: bool, directory: bool) -> Optional[str]:
    return privacy.admitted_model_path(
        params.get(field), engine=ENGINE, field=field, required=required, expect_directory=directory
    )


# Fields whose value reaches sherpa-onnx's native layer as a path it opens itself.
_NATIVE_PATH_FIELDS = (
    ("modelPath", errors.MODEL_PATH_ENCODING_UNSUPPORTED),
    ("acousticModelPath", errors.MODEL_PATH_ENCODING_UNSUPPORTED),
    ("vocoderPath", errors.MODEL_PATH_ENCODING_UNSUPPORTED),
    ("tokensPath", errors.MODEL_PATH_ENCODING_UNSUPPORTED),
    ("voicesPath", errors.MODEL_PATH_ENCODING_UNSUPPORTED),
    ("lexiconPath", errors.MODEL_PATH_ENCODING_UNSUPPORTED),
    ("dataDir", errors.TTS_DATA_PATH_ENCODING_UNSUPPORTED),
    ("dictDir", errors.TTS_DATA_PATH_ENCODING_UNSUPPORTED),
)


def _refuse_unopenable_paths(params: Dict[str, Any], resolved: Dict[str, Any]) -> None:
    """Refuse a path this build of sherpa-onnx cannot open, before it tries.

    Measured on Windows: given a data directory outside the active code page, espeak-ng fails to
    open `phontab`, falls back to its compiled-in `/usr/share/espeak-ng-data`, and calls ``exit()``.
    The worker dies with no exception to classify and no frame to attribute it to, so the host sees
    an unexplained crash rather than a configuration problem.

    This is the canary's answer encoded, not a blanket rule: the same check is deliberately absent
    for faster-whisper, whose CTranslate2 backend opens these paths correctly. Revisit per field if
    a future sherpa-onnx release fixes its path handling.
    """

    version = package_version()
    for field, code in _NATIVE_PATH_FIELDS:
        raw = params.get(field)
        if not raw or str(raw).isascii():
            continue
        raise errors.path_encoding_error(
            code,
            engine=ENGINE,
            field=field,
            path=str(raw),
            package_version=version,
        )

    # No pre-check for a missing phonemizer data directory, deliberately. Given a vits model with
    # neither a data directory nor a lexicon, sherpa-onnx prints "Not a model using characters as
    # modeling unit" and calls `exit()` -- but that message names the condition exactly: a model
    # that *does* use characters needs neither, and refusing here on shape alone rejects those
    # configurations too. Proven by doing it: the refusal broke a supervisor test whose voice is
    # configured that way and works. `TTS_PHONEMIZER_DATA_UNAVAILABLE` therefore has to be raised
    # from the crash the supervisor observes, not guessed at from the profile.


def _validated_paths(params: Dict[str, Any]) -> Dict[str, Any]:
    """Resolve exactly the files the configured ``modelKind`` requires."""

    kind = str(params.get("modelKind") or "")
    if kind not in MODEL_KINDS:
        raise errors.WorkerError(errors.MODEL_NOT_CONFIGURED, engine=ENGINE, field="modelKind")

    resolved: Dict[str, Any] = {"modelKind": kind}
    resolved["tokens"] = _model_path(params, "tokensPath", required=True, directory=False)
    resolved["lexicon"] = _model_path(params, "lexiconPath", required=False, directory=False) or ""
    resolved["dataDir"] = _model_path(params, "dataDir", required=False, directory=True) or ""
    resolved["dictDir"] = _model_path(params, "dictDir", required=False, directory=True) or ""
    _refuse_unopenable_paths(params, resolved)

    if kind == "matcha":
        # matcha splits acoustic model and vocoder; `modelPath` is the acoustic half.
        resolved["acousticModel"] = _model_path(params, "modelPath", required=True, directory=False)
        resolved["vocoder"] = _model_path(params, "vocoderPath", required=True, directory=False)
    elif kind == "kokoro":
        resolved["model"] = _model_path(params, "modelPath", required=True, directory=False)
        resolved["voices"] = _model_path(params, "voicesPath", required=True, directory=False)
    else:
        resolved["model"] = _model_path(params, "modelPath", required=True, directory=False)

    rule_fsts: List[str] = []
    for entry in params.get("ruleFsts") or []:
        resolved_entry = privacy.admitted_model_path(
            entry, engine=ENGINE, field="ruleFsts", required=True, expect_directory=False
        )
        if resolved_entry:
            rule_fsts.append(resolved_entry)
    resolved["ruleFsts"] = rule_fsts
    return resolved


def _cache_key(paths: Dict[str, Any], num_threads: int, provider: str) -> str:
    parts = [str(paths.get(key, "")) for key in sorted(paths) if key != "ruleFsts"]
    parts.append(",".join(paths.get("ruleFsts", [])))
    parts.append(str(num_threads))
    parts.append(provider)
    return "|".join(parts)


def _build_engine(params: Dict[str, Any], paths: Dict[str, Any]) -> Any:
    try:
        import sherpa_onnx  # noqa: PLC0415
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc

    num_threads = max(1, min(16, int(params.get("numThreads") or 1)))
    device = str(params.get("device") or "cpu")
    provider = "cuda" if device == "cuda" else "cpu"
    kind = paths["modelKind"]

    try:
        if kind == "kokoro":
            model_config = sherpa_onnx.OfflineTtsModelConfig(
                kokoro=sherpa_onnx.OfflineTtsKokoroModelConfig(
                    model=paths["model"],
                    voices=paths["voices"],
                    tokens=paths["tokens"],
                    data_dir=paths["dataDir"],
                    lexicon=paths["lexicon"],
                    dict_dir=paths["dictDir"],
                ),
                provider=provider,
                num_threads=num_threads,
            )
        elif kind == "matcha":
            model_config = sherpa_onnx.OfflineTtsModelConfig(
                matcha=sherpa_onnx.OfflineTtsMatchaModelConfig(
                    acoustic_model=paths["acousticModel"],
                    vocoder=paths["vocoder"],
                    tokens=paths["tokens"],
                    data_dir=paths["dataDir"],
                    lexicon=paths["lexicon"],
                    dict_dir=paths["dictDir"],
                ),
                provider=provider,
                num_threads=num_threads,
            )
        else:
            # piper models are consumed through the vits configuration in sherpa-onnx.
            model_config = sherpa_onnx.OfflineTtsModelConfig(
                vits=sherpa_onnx.OfflineTtsVitsModelConfig(
                    model=paths["model"],
                    tokens=paths["tokens"],
                    lexicon=paths["lexicon"],
                    data_dir=paths["dataDir"],
                    dict_dir=paths["dictDir"],
                ),
                provider=provider,
                num_threads=num_threads,
            )
        config = sherpa_onnx.OfflineTtsConfig(
            model=model_config,
            rule_fsts=",".join(paths["ruleFsts"]),
            max_num_sentences=1,
        )
        return sherpa_onnx.OfflineTts(config)
    except errors.WorkerError:
        raise
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc


def _engine_for(params: Dict[str, Any]) -> Any:
    paths = _validated_paths(params)
    num_threads = max(1, min(16, int(params.get("numThreads") or 1)))
    provider = "cuda" if str(params.get("device") or "cpu") == "cuda" else "cpu"
    key = _cache_key(paths, num_threads, provider)
    with _engine_lock:
        cached = _engine_cache.get(key)
    if cached is not None:
        return cached
    engine = _build_engine(params, paths)
    with _engine_lock:
        _engine_cache[key] = engine
    return engine


def probe(params: Dict[str, Any]) -> Dict[str, Any]:
    version = package_version()
    if version is None:
        raise errors.WorkerError(errors.ENGINE_IMPORT_FAILED, engine=ENGINE)
    engine = _engine_for(params)
    sample_rate = int(getattr(engine, "sample_rate", 0) or 0)
    speakers = int(getattr(engine, "num_speakers", 1) or 1)
    return {
        "engine": ENGINE,
        "packageVersion": version,
        "modelKind": str(params.get("modelKind") or ""),
        "sampleRate": sample_rate,
        "modelIdentity": f"{params.get('modelKind')}:{speakers}",
        "ready": True,
    }


def _write_wav(path: str, samples, sample_rate: int) -> int:
    """Write mono 16-bit PCM without pulling in numpy.

    ``samples`` may be a numpy array or a plain sequence of floats in ``[-1, 1]``; both are handled
    through the buffer protocol path first and the generic path second.
    """

    pcm = array.array("h")
    tolist = getattr(samples, "tolist", None)
    values = tolist() if callable(tolist) else list(samples)
    for value in values:
        scaled = int(max(-1.0, min(1.0, float(value))) * 32767.0)
        pcm.append(scaled)
    with wave.open(path, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(sample_rate)
        handle.writeframes(pcm.tobytes())
    return len(pcm)


def synthesize(params: Dict[str, Any], cancel: threading.Event) -> Dict[str, Any]:
    text = params.get("text")
    if not isinstance(text, str) or not text.strip():
        raise errors.WorkerError(errors.TTS_TEXT_TOO_LONG, engine=ENGINE, actual=0)
    if len(text) > MAX_TEXT_CODE_POINTS:
        raise errors.WorkerError(
            errors.TTS_TEXT_TOO_LONG,
            engine=ENGINE,
            limit=MAX_TEXT_CODE_POINTS,
            actual=len(text),
        )

    output_path = privacy.admitted_media_path(
        params.get("outputPath"), engine=ENGINE, must_exist=False
    )
    engine = _engine_for(params)
    if cancel.is_set():
        raise errors.WorkerError(errors.OPERATION_CANCELLED, engine=ENGINE)

    speaker_id = max(0, int(params.get("speakerId") or 0))
    speed = float(params.get("speed") or 1.0)
    if not 0.5 <= speed <= 2.0:
        raise errors.WorkerError(errors.DEVICE_CONFIGURATION_INVALID, engine=ENGINE, field="speed")

    try:
        audio = engine.generate(text, sid=speaker_id, speed=speed)
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc

    if cancel.is_set():
        raise errors.WorkerError(errors.OPERATION_CANCELLED, engine=ENGINE)

    sample_rate = int(getattr(audio, "sample_rate", 0) or 0)
    samples = getattr(audio, "samples", None)
    if sample_rate <= 0 or samples is None:
        raise errors.WorkerError(errors.ENGINE_UNAVAILABLE, engine=ENGINE, field="audio")

    try:
        sample_count = _write_wav(output_path, samples, sample_rate)
    except OSError as exc:
        raise errors.WorkerError(errors.TEMP_STORAGE_FAILED, engine=ENGINE) from exc

    return {
        "audioPath": output_path,
        "sampleRate": sample_rate,
        "sampleCount": sample_count,
        "durationMs": int(sample_count * 1000 / sample_rate) if sample_rate else 0,
        "engineVersion": package_version(),
    }


def shutdown() -> None:
    with _engine_lock:
        _engine_cache.clear()
