"""Path confinement and diagnostic redaction for the worker process.

The Rust host is the authority on confinement; this module repeats the check on the worker side so
that a host bug cannot turn into arbitrary read/write through a long-lived Python process. The two
checks are deliberately redundant rather than layered.
"""

from __future__ import annotations

import os
import sys
from typing import Optional

from . import errors

# Media paths the host hands over always live under the operation temp root. Model paths do not --
# they are user-configured directories elsewhere on disk -- so they get existence/type validation
# instead of containment.
_MEDIA_ROOT_ENV = "VANEHUB_LOCAL_MEDIA_ROOT"


def media_root() -> Optional[str]:
    root = os.environ.get(_MEDIA_ROOT_ENV)
    if not root:
        return None
    return os.path.realpath(root)


def _is_within(root: str, candidate: str) -> bool:
    try:
        return os.path.commonpath([root, candidate]) == root
    except ValueError:
        # Different drives on Windows raise rather than returning a non-match.
        return False


def admitted_media_path(raw: str, *, engine: str, must_exist: bool) -> str:
    """Resolve a host-supplied media path, requiring it to sit under the operation temp root."""

    if not isinstance(raw, str) or not raw:
        raise errors.WorkerError(errors.INPUT_NOT_FOUND, engine=engine, field="path")
    if not os.path.isabs(raw):
        raise errors.WorkerError(errors.INPUT_NOT_FOUND, engine=engine, field="path")
    resolved = os.path.realpath(raw)
    root = media_root()
    if root is not None and not _is_within(root, resolved):
        raise errors.WorkerError(errors.INPUT_NOT_FOUND, engine=engine, field="path")
    if must_exist:
        if not os.path.exists(resolved):
            raise errors.WorkerError(errors.INPUT_NOT_FOUND, engine=engine, field="path")
        if not os.path.isfile(resolved):
            raise errors.WorkerError(errors.UNSUPPORTED_MEDIA_TYPE, engine=engine, field="path")
    else:
        parent = os.path.dirname(resolved)
        if not os.path.isdir(parent):
            raise errors.WorkerError(errors.TEMP_STORAGE_FAILED, engine=engine, field="path")
    return resolved


def admitted_model_path(
    raw: Optional[str],
    *,
    engine: str,
    field: str,
    required: bool,
    expect_directory: bool,
) -> Optional[str]:
    """Resolve a user-configured model path without requiring containment."""

    if raw is None or raw == "":
        if required:
            raise errors.WorkerError(errors.MODEL_NOT_CONFIGURED, engine=engine, field=field)
        return None
    if not isinstance(raw, str) or not os.path.isabs(raw):
        raise errors.WorkerError(errors.MODEL_NOT_CONFIGURED, engine=engine, field=field)
    resolved = os.path.realpath(raw)
    if not os.path.exists(resolved):
        raise errors.WorkerError(errors.MODEL_NOT_FOUND, engine=engine, field=field)
    if expect_directory and not os.path.isdir(resolved):
        raise errors.WorkerError(errors.MODEL_NOT_FOUND, engine=engine, field=field)
    if not expect_directory and not os.path.isfile(resolved):
        raise errors.WorkerError(errors.MODEL_NOT_FOUND, engine=engine, field=field)
    return resolved


def diagnostic(engine: str, event: str, **fields: object) -> None:
    """Emit one bounded stderr line built only from allowlisted scalars.

    Nothing here interpolates a caught exception, a path, or recognized content: the host forwards
    stderr into the unified log, and a redaction that depends on the host reading carefully is not
    a redaction.
    """

    safe = errors.sanitize_details(fields)
    parts = [f"engine={engine}", f"event={event}"]
    parts.extend(f"{key}={value}" for key, value in sorted(safe.items()))
    sys.stderr.write(" ".join(parts) + "\n")
    sys.stderr.flush()
