"""Entry point for one local-media engine worker.

    <python> -u -m vane_local_media_worker --engine faster-whisper --protocol 1

One process serves exactly one engine. The dispatch loop stays on the main thread so that a `cancel`
frame is still readable while an inference call blocks a worker thread -- that is the whole reason
inference does not run inline.
"""

from __future__ import annotations

import argparse
import sys
import threading
from typing import Any, Callable, Dict, Optional, Tuple

from . import errors, privacy, protocol

_ENGINE_MODULES = {
    protocol.ENGINE_PADDLE_OCR: "paddle_ocr_engine",
    protocol.ENGINE_FASTER_WHISPER: "faster_whisper_engine",
    protocol.ENGINE_SHERPA_ONNX: "sherpa_onnx_tts_engine",
}

_ENGINE_METHODS = {
    protocol.ENGINE_PADDLE_OCR: "ocr",
    protocol.ENGINE_FASTER_WHISPER: "transcribe",
    protocol.ENGINE_SHERPA_ONNX: "synthesize",
}


class _ActiveRequest:
    __slots__ = ("request_id", "cancel", "thread")

    def __init__(self, request_id: str, cancel: threading.Event, thread: threading.Thread) -> None:
        self.request_id = request_id
        self.cancel = cancel
        self.thread = thread


class WorkerRuntime:
    """Owns the one-inference-at-a-time invariant and the cancel handshake."""

    def __init__(self, engine: str, module: Any, writer: protocol.FrameWriter) -> None:
        self._engine = engine
        self._module = module
        self._writer = writer
        self._state_lock = threading.Lock()
        self._active: Optional[_ActiveRequest] = None
        self.stopped = threading.Event()

    def handle(self, frame: Dict[str, Any]) -> None:
        frame_type = frame["type"]
        if frame_type == protocol.FRAME_REQUEST:
            self._handle_request(frame)
        elif frame_type == protocol.FRAME_CANCEL:
            self._handle_cancel(frame)
        elif frame_type == protocol.FRAME_SHUTDOWN:
            self._handle_shutdown()

    def _resolve_method(self, method: str) -> Optional[Callable[..., Dict[str, Any]]]:
        if method == "probe":
            return lambda params, _cancel: self._module.probe(params)
        if method == _ENGINE_METHODS[self._engine]:
            return getattr(self._module, method)
        return None

    def _handle_request(self, frame: Dict[str, Any]) -> None:
        request_id: str = frame["id"]
        method: str = frame["method"]
        params: Dict[str, Any] = frame.get("params") or {}

        handler = self._resolve_method(method)
        if handler is None:
            self._writer.write(
                protocol.error_frame(
                    request_id,
                    errors.WorkerError(
                        errors.WORKER_PROTOCOL_ERROR, engine=self._engine, method=method
                    ).to_wire(),
                )
            )
            return

        cancel = threading.Event()
        with self._state_lock:
            if self._active is not None:
                self._writer.write(
                    protocol.error_frame(
                        request_id,
                        errors.WorkerError(errors.ENGINE_BUSY, engine=self._engine).to_wire(),
                    )
                )
                return
            thread = threading.Thread(
                target=self._run,
                args=(request_id, method, params, handler, cancel),
                name=f"local-media-{self._engine}",
                daemon=True,
            )
            self._active = _ActiveRequest(request_id, cancel, thread)
        thread.start()

    def _run(
        self,
        request_id: str,
        method: str,
        params: Dict[str, Any],
        handler: Callable[..., Dict[str, Any]],
        cancel: threading.Event,
    ) -> None:
        try:
            result = handler(params, cancel)
            if cancel.is_set():
                frame = protocol.error_frame(
                    request_id,
                    errors.WorkerError(errors.OPERATION_CANCELLED, engine=self._engine).to_wire(),
                )
            else:
                frame = protocol.response_frame(request_id, result)
        except BaseException as exc:  # noqa: BLE001 - a crash must still produce a mapped frame
            mapped = errors.classify_exception(exc, engine=self._engine)
            privacy.diagnostic(
                self._engine, "request_failed", method=method, exceptionType=type(exc).__name__
            )
            frame = protocol.error_frame(request_id, mapped.to_wire())
        finally:
            with self._state_lock:
                if self._active is not None and self._active.request_id == request_id:
                    self._active = None
        self._writer.write(frame)

    def _handle_cancel(self, frame: Dict[str, Any]) -> None:
        request_id = frame["id"]
        with self._state_lock:
            active = self._active
        if active is not None and active.request_id == request_id:
            active.cancel.set()
            privacy.diagnostic(self._engine, "cancel_requested")

    def _handle_shutdown(self) -> None:
        with self._state_lock:
            active = self._active
        if active is not None:
            active.cancel.set()
        try:
            self._module.shutdown()
        except Exception:  # noqa: BLE001 - shutdown is best effort
            privacy.diagnostic(self._engine, "shutdown_cleanup_failed")
        self.stopped.set()


def _load_engine_module(engine: str) -> Any:
    module_name = _ENGINE_MODULES[engine]
    package = __name__.rsplit(".", 1)[0]
    return __import__(f"{package}.{module_name}", fromlist=[module_name])


def _parse_args(argv) -> Tuple[str, int]:
    parser = argparse.ArgumentParser(prog="vane-local-media-worker", add_help=False)
    parser.add_argument("--engine", required=True, choices=list(protocol.SUPPORTED_ENGINES))
    parser.add_argument("--protocol", required=True, type=int)
    parsed = parser.parse_args(argv)
    return parsed.engine, parsed.protocol


def main(argv=None) -> int:
    engine, protocol_version = _parse_args(sys.argv[1:] if argv is None else argv)
    if protocol_version != protocol.PROTOCOL_VERSION:
        privacy.diagnostic(engine, "unsupported_protocol")
        return 2

    stream = protocol.install_stdout_guard()
    writer = protocol.FrameWriter(stream)

    try:
        module = _load_engine_module(engine)
    except Exception:  # noqa: BLE001
        privacy.diagnostic(engine, "engine_module_load_failed")
        return 3

    runtime = WorkerRuntime(engine, module, writer)
    writer.write(protocol.hello_frame(engine, module.CAPABILITIES, module.package_version()))

    try:
        for frame in protocol.read_frames(sys.stdin):
            runtime.handle(frame)
            if runtime.stopped.is_set():
                break
    except protocol.ProtocolError:
        # A malformed inbound frame is unrecoverable: the host cannot know which request the stream
        # is now aligned to, so the worker exits and lets the supervisor start a clean one.
        privacy.diagnostic(engine, "inbound_protocol_error")
        return 4
    except KeyboardInterrupt:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
