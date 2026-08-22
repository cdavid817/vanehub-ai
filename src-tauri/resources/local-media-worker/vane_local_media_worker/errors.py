"""Stable local-media error codes shared by the Rust host and this bridge.

The host localizes by ``code``; it never renders text produced here. Anything that would carry a
path, a traceback, or recognized content is deliberately absent from the wire error -- ``details``
is an allowlist of scalars, not a place to smuggle a message.
"""

from __future__ import annotations

from typing import Any, Dict, Mapping, Optional

PROFILE_REVISION_CONFLICT = "PROFILE_REVISION_CONFLICT"
ENGINE_UNCONFIGURED = "ENGINE_UNCONFIGURED"
ENGINE_BUSY = "ENGINE_BUSY"
ENGINE_UNAVAILABLE = "ENGINE_UNAVAILABLE"
ENGINE_IMPORT_FAILED = "ENGINE_IMPORT_FAILED"
ENGINE_VERSION_UNSUPPORTED = "ENGINE_VERSION_UNSUPPORTED"
MODEL_NOT_CONFIGURED = "MODEL_NOT_CONFIGURED"
MODEL_NOT_FOUND = "MODEL_NOT_FOUND"
MODEL_INCOMPATIBLE = "MODEL_INCOMPATIBLE"
MODEL_DOWNLOAD_BLOCKED = "MODEL_DOWNLOAD_BLOCKED"
DEVICE_CONFIGURATION_INVALID = "DEVICE_CONFIGURATION_INVALID"
INPUT_NOT_FOUND = "INPUT_NOT_FOUND"
INPUT_TOO_LARGE = "INPUT_TOO_LARGE"
UNSUPPORTED_MEDIA_TYPE = "UNSUPPORTED_MEDIA_TYPE"
PDF_PAGE_LIMIT_EXCEEDED = "PDF_PAGE_LIMIT_EXCEEDED"
IMAGE_PIXEL_LIMIT_EXCEEDED = "IMAGE_PIXEL_LIMIT_EXCEEDED"
NO_TEXT_DETECTED = "NO_TEXT_DETECTED"
NO_SPEECH_DETECTED = "NO_SPEECH_DETECTED"
TTS_TEXT_TOO_LONG = "TTS_TEXT_TOO_LONG"
OPERATION_CANCELLED = "OPERATION_CANCELLED"
WORKER_PROTOCOL_ERROR = "WORKER_PROTOCOL_ERROR"
WORKER_CRASHED = "WORKER_CRASHED"
TEMP_STORAGE_FAILED = "TEMP_STORAGE_FAILED"

_MESSAGE_KEYS = {
    PROFILE_REVISION_CONFLICT: "localMedia.errors.profileRevisionConflict",
    ENGINE_UNCONFIGURED: "localMedia.errors.engineUnconfigured",
    ENGINE_BUSY: "localMedia.errors.engineBusy",
    ENGINE_UNAVAILABLE: "localMedia.errors.engineUnavailable",
    ENGINE_IMPORT_FAILED: "localMedia.errors.engineImportFailed",
    ENGINE_VERSION_UNSUPPORTED: "localMedia.errors.engineVersionUnsupported",
    MODEL_NOT_CONFIGURED: "localMedia.errors.modelNotConfigured",
    MODEL_NOT_FOUND: "localMedia.errors.modelNotFound",
    MODEL_INCOMPATIBLE: "localMedia.errors.modelIncompatible",
    MODEL_DOWNLOAD_BLOCKED: "localMedia.errors.modelDownloadBlocked",
    DEVICE_CONFIGURATION_INVALID: "localMedia.errors.deviceConfigurationInvalid",
    INPUT_NOT_FOUND: "localMedia.errors.inputNotFound",
    INPUT_TOO_LARGE: "localMedia.errors.inputTooLarge",
    UNSUPPORTED_MEDIA_TYPE: "localMedia.errors.unsupportedMediaType",
    PDF_PAGE_LIMIT_EXCEEDED: "localMedia.errors.pdfPageLimitExceeded",
    IMAGE_PIXEL_LIMIT_EXCEEDED: "localMedia.errors.imagePixelLimitExceeded",
    NO_TEXT_DETECTED: "localMedia.errors.noTextDetected",
    NO_SPEECH_DETECTED: "localMedia.errors.noSpeechDetected",
    TTS_TEXT_TOO_LONG: "localMedia.errors.ttsTextTooLong",
    OPERATION_CANCELLED: "localMedia.errors.operationCancelled",
    WORKER_PROTOCOL_ERROR: "localMedia.errors.workerProtocolError",
    WORKER_CRASHED: "localMedia.errors.workerCrashed",
    TEMP_STORAGE_FAILED: "localMedia.errors.tempStorageFailed",
}

_RETRYABLE = frozenset({ENGINE_BUSY, WORKER_CRASHED, TEMP_STORAGE_FAILED})

# Only these keys may appear in `safeDetails`, and only with scalar values. Everything else is
# dropped rather than truncated: a partially redacted path is still a path.
_ALLOWED_DETAIL_KEYS = frozenset(
    {
        "engine",
        "method",
        "field",
        "limit",
        "actual",
        "pageCount",
        "sampleRate",
        "durationMs",
        "exceptionType",
        "modelKind",
        "device",
    }
)


class WorkerError(Exception):
    """An engine failure that maps to a stable code instead of a rendered message."""

    def __init__(self, code: str, **details: Any) -> None:
        super().__init__(code)
        self.code = code
        self.details = sanitize_details(details)

    def to_wire(self) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "code": self.code,
            "messageKey": _MESSAGE_KEYS.get(self.code, "localMedia.errors.unknown"),
            "retryable": self.code in _RETRYABLE,
        }
        if self.details:
            payload["safeDetails"] = self.details
        return payload


def sanitize_details(details: Optional[Mapping[str, Any]]) -> Dict[str, Any]:
    """Drop every detail that is not an allowlisted key holding a bounded scalar."""

    if not details:
        return {}
    safe: Dict[str, Any] = {}
    for key, value in details.items():
        if key not in _ALLOWED_DETAIL_KEYS:
            continue
        if isinstance(value, bool) or isinstance(value, int) or isinstance(value, float):
            safe[key] = value
        elif isinstance(value, str) and len(value) <= 64 and value.isprintable():
            safe[key] = value
    return safe


def classify_exception(exc: BaseException, *, engine: str) -> WorkerError:
    """Map an arbitrary Python exception onto a stable code without keeping its text.

    Only the exception's class name survives, and only after it passes the same printable/length
    filter as any other detail. The message is discarded here rather than at the host boundary --
    a model loader routinely puts the model path into ``str(exc)``.
    """

    if isinstance(exc, WorkerError):
        return exc
    if isinstance(exc, ModuleNotFoundError):
        return WorkerError(ENGINE_IMPORT_FAILED, engine=engine, exceptionType=type(exc).__name__)
    if isinstance(exc, ImportError):
        return WorkerError(ENGINE_IMPORT_FAILED, engine=engine, exceptionType=type(exc).__name__)
    if isinstance(exc, FileNotFoundError):
        return WorkerError(MODEL_NOT_FOUND, engine=engine, exceptionType=type(exc).__name__)
    if isinstance(exc, PermissionError):
        return WorkerError(MODEL_NOT_FOUND, engine=engine, exceptionType=type(exc).__name__)
    if isinstance(exc, OSError) and _looks_like_blocked_network(exc):
        return WorkerError(MODEL_DOWNLOAD_BLOCKED, engine=engine, exceptionType=type(exc).__name__)
    if isinstance(exc, MemoryError):
        return WorkerError(ENGINE_UNAVAILABLE, engine=engine, exceptionType=type(exc).__name__)
    return WorkerError(ENGINE_UNAVAILABLE, engine=engine, exceptionType=type(exc).__name__)


def _looks_like_blocked_network(exc: BaseException) -> bool:
    """Recognize the shape of a denied socket without reading the exception's text.

    Offline-enforcement tests replace ``socket.socket`` with a raiser; libraries that try to reach a
    hub surface that as an ``OSError`` chain. Matching on type rather than message keeps this from
    depending on a locale-dependent errno string.
    """

    seen = 0
    current: Optional[BaseException] = exc
    while current is not None and seen < 8:
        if type(current).__name__ in {
            "NetworkAccessDeniedError",
            "ConnectionRefusedError",
            "ConnectionError",
            "URLError",
            "SSLError",
            "Timeout",
            "ConnectTimeout",
            "ProxyError",
        }:
            return True
        current = current.__cause__ or current.__context__
        seen += 1
    return False
