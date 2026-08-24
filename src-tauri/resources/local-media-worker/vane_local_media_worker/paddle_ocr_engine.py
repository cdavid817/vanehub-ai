"""PaddleOCR bridge.

Model configuration is always explicit. PaddleOCR resolves an omitted model argument by downloading
an official model, so every optional sub-model is passed as ``False`` unless its local directory was
configured -- omission is not neutral here, it is a download.
"""

from __future__ import annotations

import threading
from typing import Any, Dict, List, Optional

from . import errors, privacy

ENGINE = "paddleocr"
CAPABILITIES = ["probe", "ocr", "cancel", "shutdown"]

_LINE_CHARACTER_CEILING = 20_000
_engine_cache: Dict[str, Any] = {}
_engine_lock = threading.Lock()


def package_version() -> Optional[str]:
    try:
        import paddleocr  # noqa: PLC0415 - lazy so a probe failure is reportable, not fatal

        version = getattr(paddleocr, "__version__", None)
        return str(version) if version else None
    except Exception:  # noqa: BLE001 - any import problem is a probe result, not a crash
        return None


def _resolve_device(device: str) -> str:
    if device == "cpu":
        return "cpu"
    if device == "cuda":
        return "gpu"
    if device == "auto":
        return "gpu" if _cuda_visible() else "cpu"
    raise errors.WorkerError(errors.DEVICE_CONFIGURATION_INVALID, engine=ENGINE, device=device)


def _cuda_visible() -> bool:
    try:
        import paddle  # noqa: PLC0415

        return bool(paddle.device.is_compiled_with_cuda() and paddle.device.cuda.device_count() > 0)
    except Exception:  # noqa: BLE001
        return False


def _build_engine(params: Dict[str, Any]) -> Any:
    try:
        from paddleocr import PaddleOCR  # noqa: PLC0415
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_exception(exc, engine=ENGINE) from exc

    paddlex_config = privacy.admitted_model_path(
        params.get("paddleXConfigPath"),
        engine=ENGINE,
        field="paddleXConfigPath",
        required=False,
        expect_directory=False,
    )
    detection_dir = privacy.admitted_model_path(
        params.get("textDetectionModelDir"),
        engine=ENGINE,
        field="textDetectionModelDir",
        required=False,
        expect_directory=True,
    )
    recognition_dir = privacy.admitted_model_path(
        params.get("textRecognitionModelDir"),
        engine=ENGINE,
        field="textRecognitionModelDir",
        required=False,
        expect_directory=True,
    )
    orientation_dir = privacy.admitted_model_path(
        params.get("textLineOrientationModelDir"),
        engine=ENGINE,
        field="textLineOrientationModelDir",
        required=False,
        expect_directory=True,
    )

    if paddlex_config is None and (detection_dir is None or recognition_dir is None):
        raise errors.WorkerError(errors.MODEL_NOT_CONFIGURED, engine=ENGINE, field="models")

    device = _resolve_device(str(params.get("device", "auto")))
    language = str(params.get("language") or "ch")

    kwargs: Dict[str, Any] = {
        "device": device,
        # Every optional stage is off unless its local directory was configured. Leaving one at its
        # default would make PaddleOCR fetch the official checkpoint on first use.
        "use_doc_orientation_classify": False,
        "use_doc_unwarping": False,
        "use_textline_orientation": orientation_dir is not None,
    }
    # Passed to the constructor, which is what reaches every pipeline stage. The global
    # `FLAGS_use_mkldnn` was measured and does nothing here: PaddleX builds its runners from its own
    # pipeline configuration and never reads it, so a process-wide setting would look right in
    # review and be silently ignored at runtime.
    acceleration = str(params.get("cpuAcceleration") or "library-default")
    if acceleration == "disabled":
        kwargs["enable_mkldnn"] = False
    elif acceleration == "enabled":
        kwargs["enable_mkldnn"] = True
    if orientation_dir is not None:
        kwargs["textline_orientation_model_dir"] = orientation_dir
    if paddlex_config is not None:
        kwargs["paddlex_config"] = paddlex_config
    else:
        kwargs["text_detection_model_dir"] = detection_dir
        kwargs["text_recognition_model_dir"] = recognition_dir
        kwargs["lang"] = language

    try:
        return PaddleOCR(**kwargs)
    except Exception as exc:  # noqa: BLE001
        raise errors.classify_vendor_exception(
            exc,
            engine=ENGINE,
            field="textDetectionModelDir",
            path=detection_dir or paddlex_config,
            package_version=package_version(),
            acceleration=acceleration,
        ) from exc


def _cache_key(params: Dict[str, Any]) -> str:
    return "|".join(
        str(params.get(field, ""))
        for field in (
            "paddleXConfigPath",
            "textDetectionModelDir",
            "textRecognitionModelDir",
            "textLineOrientationModelDir",
            "language",
            "device",
        )
    )


def _engine_for(params: Dict[str, Any]) -> Any:
    key = _cache_key(params)
    with _engine_lock:
        cached = _engine_cache.get(key)
        if cached is not None:
            return cached
    engine = _build_engine(params)
    with _engine_lock:
        # A concurrent build cannot happen (one inference per worker) but the cache is still keyed
        # rather than overwritten so a profile change does not silently reuse the old engine.
        _engine_cache[key] = engine
    return engine


def probe(params: Dict[str, Any]) -> Dict[str, Any]:
    version = package_version()
    if version is None:
        raise errors.WorkerError(errors.ENGINE_IMPORT_FAILED, engine=ENGINE)
    engine = _engine_for(params)
    return {
        "engine": ENGINE,
        "packageVersion": version,
        "device": _resolve_device(str(params.get("device", "auto"))),
        "language": str(params.get("language") or "ch"),
        "modelIdentity": _model_identity(params),
        "ready": engine is not None,
    }


def _model_identity(params: Dict[str, Any]) -> str:
    """A stable label that identifies the configuration without disclosing where it lives."""

    if params.get("paddleXConfigPath"):
        return "paddlex-config"
    return f"det+rec:{params.get('language') or 'ch'}"


def _extract_lines(page_result: Any) -> List[Dict[str, Any]]:
    """Normalize one page of PaddleOCR output into ordered ``{text, confidence}`` records.

    PaddleOCR 3.x ``predict`` yields mapping-like results carrying ``rec_texts``/``rec_scores``.
    Older list-of-``[polygon, (text, score)]`` output is accepted too, because a user-provided
    environment is not version-pinned by this application.
    """

    lines: List[Dict[str, Any]] = []
    texts = None
    scores = None
    polygons = None
    if hasattr(page_result, "get"):
        texts = page_result.get("rec_texts")
        scores = page_result.get("rec_scores")
        polygons = page_result.get("rec_polys") or page_result.get("dt_polys")
    if texts is None and hasattr(page_result, "__getitem__"):
        try:
            texts = page_result["rec_texts"]
            scores = page_result["rec_scores"]
            polygons = page_result["dt_polys"]
        except Exception:  # noqa: BLE001
            texts = None
    if texts is not None:
        for index, text in enumerate(texts):
            cleaned = _clean(text)
            if not cleaned:
                continue
            lines.append(
                {
                    "text": cleaned,
                    "confidence": _confidence_at(scores, index),
                    "polygon": _polygon_at(polygons, index),
                }
            )
        return lines

    if isinstance(page_result, list):
        for entry in page_result:
            if not isinstance(entry, (list, tuple)) or len(entry) < 2:
                continue
            payload = entry[1]
            if isinstance(payload, (list, tuple)) and payload:
                text = _clean(payload[0])
                if not text:
                    continue
                confidence = None
                if len(payload) > 1:
                    try:
                        confidence = float(payload[1])
                    except (TypeError, ValueError):
                        confidence = None
                lines.append(
                    {"text": text, "confidence": confidence, "polygon": _polygon(entry[0])}
                )
    return lines


def _confidence_at(scores: Any, index: int) -> Optional[float]:
    if scores is None:
        return None
    try:
        return float(scores[index])
    except (TypeError, ValueError, IndexError, KeyError):
        return None


def _polygon_at(polygons: Any, index: int) -> Optional[List[List[float]]]:
    if polygons is None:
        return None
    try:
        return _polygon(polygons[index])
    except (TypeError, IndexError, KeyError):
        return None


def _polygon(raw: Any) -> Optional[List[List[float]]]:
    """Normalize a bounding polygon to a plain list of ``[x, y]`` pairs.

    PaddleOCR returns numpy arrays here. Converting explicitly keeps `json.dumps` from failing on a
    type it does not know, which would turn a successful page into a protocol error.
    """

    if raw is None:
        return None
    tolist = getattr(raw, "tolist", None)
    points = tolist() if callable(tolist) else raw
    if not isinstance(points, (list, tuple)) or len(points) < 3:
        return None
    normalized: List[List[float]] = []
    for point in points:
        pair = point.tolist() if hasattr(point, "tolist") else point
        if not isinstance(pair, (list, tuple)) or len(pair) < 2:
            return None
        try:
            normalized.append([float(pair[0]), float(pair[1])])
        except (TypeError, ValueError):
            return None
    return normalized


def _clean(value: Any) -> str:
    if value is None:
        return ""
    text = str(value).replace("\r\n", "\n").replace("\r", "\n").replace("\x00", "")
    if len(text) > _LINE_CHARACTER_CEILING:
        text = text[:_LINE_CHARACTER_CEILING]
    return text.strip()


def ocr(params: Dict[str, Any], cancel: threading.Event) -> Dict[str, Any]:
    source_path = privacy.admitted_media_path(
        params.get("sourcePath"), engine=ENGINE, must_exist=True
    )
    max_pages = int(params.get("maxPdfPages") or 20)
    max_characters = int(params.get("maxOutputCharacters") or 200_000)

    engine = _engine_for(params)
    if cancel.is_set():
        raise errors.WorkerError(errors.OPERATION_CANCELLED, engine=ENGINE)

    try:
        raw = engine.predict(input=source_path)
    except TypeError:
        # Some builds expose only the positional form.
        raw = engine.predict(source_path)
    except Exception as exc:  # noqa: BLE001
        # The acceleration incompatibility surfaces here rather than at construction: the graph is
        # accepted on load and only fails when an operator actually runs.
        raise errors.classify_vendor_exception(
            exc,
            engine=ENGINE,
            field="textDetectionModelDir",
            path=params.get("textDetectionModelDir"),
            package_version=package_version(),
            acceleration=str(params.get("cpuAcceleration") or "library-default"),
        ) from exc

    if raw is None:
        raw = []
    if not isinstance(raw, list):
        raw = [raw]
    if len(raw) > max_pages:
        raise errors.WorkerError(
            errors.PDF_PAGE_LIMIT_EXCEEDED, engine=ENGINE, limit=max_pages, actual=len(raw)
        )

    pages: List[Dict[str, Any]] = []
    total_characters = 0
    truncated = False
    for index, page_result in enumerate(raw):
        if cancel.is_set():
            raise errors.WorkerError(errors.OPERATION_CANCELLED, engine=ENGINE)
        lines = _extract_lines(page_result)
        page_text_parts: List[str] = []
        admitted_lines: List[Dict[str, Any]] = []
        for line in lines:
            if total_characters + len(line["text"]) > max_characters:
                truncated = True
                break
            total_characters += len(line["text"]) + 1
            page_text_parts.append(line["text"])
            admitted_lines.append(line)
        pages.append(
            {
                "pageNumber": index + 1,
                "text": "\n".join(page_text_parts),
                "lineCount": len(page_text_parts),
                # Per-line detail for the OnePiece tool contract. The composer reads only `text`;
                # the two entry points share one result, so the richer form travels with it.
                "lines": admitted_lines,
            }
        )
        if truncated:
            break

    recognized = any(page["lineCount"] > 0 for page in pages)
    return {
        "pages": pages,
        "pageCount": len(pages),
        "characterCount": total_characters,
        "truncated": truncated,
        "noTextDetected": not recognized,
        "engineVersion": package_version(),
        "modelIdentity": _model_identity(params),
    }


def shutdown() -> None:
    with _engine_lock:
        _engine_cache.clear()
