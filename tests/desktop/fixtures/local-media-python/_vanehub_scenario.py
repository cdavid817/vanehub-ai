"""Scenario control shared by the three test-only inference packages.

These packages stand in for `paddleocr`, `faster_whisper`, and `sherpa_onnx` so that the *real*
`vane_local_media_worker` -- its dispatch loop, JSON Lines protocol, engine wrappers, result
conversion, and error mapping -- runs unmodified against deterministic inference. Only the
third-party libraries are replaced.

Standard library only. Nothing here opens a socket, downloads anything, or loads a model.
"""

from __future__ import annotations

import json
import os
import time
from typing import Any, Dict

SCENARIO_ENV = "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE"

SUCCESS = "success"
ENGINE_ERROR = "engine_error"
EMPTY = "empty"
DELAY = "delay"
CRASH_ONCE = "crash_once"
HANG = "hang"
MALFORMED_RESULT = "malformed_result"

#: Exit code for `crash_once`. Distinctive so the supervisor's crash detection is provably reacting
#: to a real child exit rather than to a protocol error the worker reported politely.
CRASH_EXIT_CODE = 17


def _scenario_path() -> str | None:
    return os.environ.get(SCENARIO_ENV) or None


def load(engine: str) -> Dict[str, Any]:
    """Read this engine's scenario, defaulting to plain success.

    Re-read on every call rather than cached: a spec changes the scenario between requests to the
    same long-lived worker, which is exactly how restart-then-succeed is exercised.
    """

    path = _scenario_path()
    if not path or not os.path.isfile(path):
        return {"behaviour": SUCCESS}
    try:
        with open(path, encoding="utf-8") as handle:
            document = json.load(handle)
    except (OSError, ValueError):
        return {"behaviour": SUCCESS}
    entry = document.get(engine)
    if not isinstance(entry, dict):
        return {"behaviour": SUCCESS}
    return entry


def _marker(engine: str, name: str) -> str | None:
    path = _scenario_path()
    if not path:
        return None
    return f"{path}.{engine}.{name}"


def _write_marker(marker: str | None) -> None:
    if not marker:
        return
    try:
        with open(marker, "w", encoding="utf-8") as handle:
            handle.write("1")
    except OSError:
        pass


def apply_common(engine: str, scenario: Dict[str, Any]) -> str:
    """Run the behaviours that are identical across engines; return the remaining behaviour.

    `crash_once` and `hang` are handled here because both are about the *process*, and the point of
    this fixture is that the host observes a real child dying or a real child refusing to answer --
    not a polite error frame describing one.
    """

    behaviour = str(scenario.get("behaviour") or SUCCESS)

    if behaviour == CRASH_ONCE:
        marker = _marker(engine, "crashed")
        already = bool(marker and os.path.exists(marker))
        if not already:
            _write_marker(marker)
            # Not an exception: an exception would be caught by the worker and reported as a mapped
            # error frame, which is the opposite of what a crash test needs. `os._exit` skips
            # cleanup and kills the interpreter, so the supervisor sees the pipe close.
            os._exit(CRASH_EXIT_CODE)
        return SUCCESS

    if behaviour == HANG:
        # Uncooperative on purpose. The worker's cancel token is only observed between engine
        # calls, so a library that never returns is what forces the grace period to expire and the
        # supervisor to kill the process.
        #
        # The first marker is what lets a spec cancel while the engine is genuinely stuck rather
        # than while it is still starting; without it, a worker that failed to launch would produce
        # the same observations as one that was killed.
        _write_marker(_marker(engine, "hang-started"))
        time.sleep(float(scenario.get("hangSeconds") or 600))
        # Only reached if nobody killed this process. Its absence after the sleep would have
        # elapsed is what distinguishes "the host killed the worker" from "the host walked away
        # and left it running" -- two outcomes that look identical from the operation's result.
        _write_marker(_marker(engine, "hang-completed"))
        return SUCCESS

    if behaviour == DELAY:
        time.sleep(float(scenario.get("delaySeconds") or 1.0))
        return SUCCESS

    return behaviour


def failure_message(scenario: Dict[str, Any]) -> str:
    return str(scenario.get("message") or "fixture engine failure")
