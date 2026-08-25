"""Version 1 of the local-media JSON Lines protocol.

One UTF-8 JSON object per line, in both directions. stdout carries protocol frames and nothing else
-- a stray ``print`` anywhere in an engine library would otherwise be indistinguishable from a
frame, which is exactly the contamination the host terminates a worker for. ``install_stdout_guard``
redirects the process-wide ``sys.stdout`` to stderr before any engine is imported so that a library
that prints cannot corrupt the stream.
"""

from __future__ import annotations

import io
import json
import sys
import threading
from typing import Any, Dict, Iterator, Optional, TextIO

PROTOCOL_VERSION = 1

# Requests carry paths and parameters, never media bytes, so 1 MiB is generous. Responses carry
# recognized text, which the host bounds separately; 8 MiB keeps a pathological OCR page from
# wedging the pipe while staying far above any product limit.
MAX_REQUEST_FRAME_BYTES = 1 * 1024 * 1024
MAX_RESPONSE_FRAME_BYTES = 8 * 1024 * 1024

FRAME_HELLO = "hello"
FRAME_REQUEST = "request"
FRAME_RESPONSE = "response"
FRAME_ERROR = "error"
FRAME_CANCEL = "cancel"
FRAME_SHUTDOWN = "shutdown"

ENGINE_PADDLE_OCR = "paddleocr"
ENGINE_FASTER_WHISPER = "faster-whisper"
ENGINE_SHERPA_ONNX = "sherpa-onnx"

SUPPORTED_ENGINES = (ENGINE_PADDLE_OCR, ENGINE_FASTER_WHISPER, ENGINE_SHERPA_ONNX)


class ProtocolError(Exception):
    """A frame the worker refuses to interpret. The host terminates the worker on this."""


class FrameWriter:
    """Serializes frames onto one stream under a lock.

    The inference thread and the stdin dispatch loop both write, so the lock is load-bearing: two
    interleaved ``write`` calls would produce a line the host parses as malformed and treats as
    stdout contamination.
    """

    def __init__(self, stream: TextIO) -> None:
        self._stream = stream
        self._lock = threading.Lock()

    def write(self, frame: Dict[str, Any]) -> None:
        payload = json.dumps(frame, ensure_ascii=False, separators=(",", ":"))
        encoded_length = len(payload.encode("utf-8"))
        if encoded_length > MAX_RESPONSE_FRAME_BYTES:
            payload = json.dumps(
                {
                    "v": PROTOCOL_VERSION,
                    "type": FRAME_RESPONSE,
                    "id": frame.get("id"),
                    "ok": False,
                    "error": {
                        "code": "WORKER_PROTOCOL_ERROR",
                        "messageKey": "localMedia.errors.workerProtocolError",
                        "retryable": False,
                        "safeDetails": {"limit": MAX_RESPONSE_FRAME_BYTES, "actual": encoded_length},
                    },
                },
                ensure_ascii=False,
                separators=(",", ":"),
            )
        with self._lock:
            self._stream.write(payload)
            self._stream.write("\n")
            self._stream.flush()


def hello_frame(engine: str, capabilities: list, package_version: Optional[str]) -> Dict[str, Any]:
    return {
        "v": PROTOCOL_VERSION,
        "type": FRAME_HELLO,
        "engine": engine,
        "workerVersion": "1",
        "packageVersion": package_version,
        "capabilities": capabilities,
    }


def response_frame(request_id: str, result: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "v": PROTOCOL_VERSION,
        "type": FRAME_RESPONSE,
        "id": request_id,
        "ok": True,
        "result": result,
    }


def error_frame(request_id: Optional[str], error: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "v": PROTOCOL_VERSION,
        "type": FRAME_RESPONSE,
        "id": request_id,
        "ok": False,
        "error": error,
    }


def read_frames(stream: TextIO) -> Iterator[Dict[str, Any]]:
    """Yield validated inbound frames, raising ``ProtocolError`` on anything malformed.

    Oversize is detected on the encoded line rather than the parsed object because the point of the
    bound is to cap what the worker is willing to buffer, and by parse time it has already been
    buffered.
    """

    for raw in stream:
        line = raw.rstrip("\r\n")
        if not line.strip():
            continue
        if len(line.encode("utf-8")) > MAX_REQUEST_FRAME_BYTES:
            raise ProtocolError("frame exceeds maximum size")
        try:
            frame = json.loads(line)
        except ValueError as exc:
            raise ProtocolError("frame is not valid JSON") from exc
        if not isinstance(frame, dict):
            raise ProtocolError("frame is not an object")
        if frame.get("v") != PROTOCOL_VERSION:
            raise ProtocolError("unsupported protocol version")
        frame_type = frame.get("type")
        if frame_type not in {FRAME_REQUEST, FRAME_CANCEL, FRAME_SHUTDOWN}:
            raise ProtocolError("unsupported inbound frame type")
        if frame_type in {FRAME_REQUEST, FRAME_CANCEL}:
            request_id = frame.get("id")
            if not isinstance(request_id, str) or not request_id or len(request_id) > 128:
                raise ProtocolError("missing or invalid request id")
        if frame_type == FRAME_REQUEST:
            method = frame.get("method")
            if not isinstance(method, str) or not method or len(method) > 64:
                raise ProtocolError("missing or invalid method")
            params = frame.get("params")
            if params is not None and not isinstance(params, dict):
                raise ProtocolError("params must be an object")
        yield frame


def install_stdout_guard() -> TextIO:
    """Take exclusive ownership of the real stdout and point ``sys.stdout`` at stderr.

    Returns the protocol stream. Engine libraries that print progress banners then land in the
    redacted stderr channel instead of terminating the worker.
    """

    protocol_stream = sys.stdout
    if isinstance(protocol_stream, io.TextIOWrapper):
        protocol_stream.reconfigure(encoding="utf-8", newline="\n")
    sys.stdout = sys.stderr
    return protocol_stream
