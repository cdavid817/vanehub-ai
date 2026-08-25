# The remote workspace helper.
#
# Shipped as data, not as a command. The bootstrap that runs this is a constant one-liner with no
# shell metacharacters; this program arrives base64-encoded on stdin and the request arrives after
# it. Nothing user-controlled is ever interpolated into a command line, which is what makes "no
# shell injection" a property of the arrangement rather than a rule somebody has to keep applying.
#
# Runs under `python3 -I -S`: isolated, no site packages, no user site directory, environment
# variables ignored. Only the standard library is used, and only the parts present in every
# supported version.
#
# Every failure is a stable code. An exception message would carry remote paths and usernames back
# into a client that logs what it receives, and the client cannot redact what it does not
# understand.

import json
import os
import shutil
import subprocess
import sys

HELPER_VERSION = 1

# What one response may be. The client bounds this too; both exist because they fail differently —
# this one stops the helper building a huge string, and the client's stops a program that is not
# this helper from filling its memory.
MAX_RESPONSE_BYTES = 1024 * 1024

# How long any one subprocess may take. A `git` that hangs on a network filesystem must not turn a
# capability probe into a stuck channel.
SUBPROCESS_TIMEOUT_SECONDS = 10


def respond(payload):
    body = json.dumps(payload, separators=(",", ":"))
    if len(body) > MAX_RESPONSE_BYTES:
        body = json.dumps(
            {"version": HELPER_VERSION, "ok": False, "reasonCode": "remote_helper_response_too_large"},
            separators=(",", ":"),
        )
    sys.stdout.write(body)
    sys.stdout.flush()
    sys.exit(0)


def refuse(reason_code):
    respond({"version": HELPER_VERSION, "ok": False, "reasonCode": reason_code})


def tool_available(name, version_argument):
    """Whether a tool is both present and runnable.

    `shutil.which` alone is not enough: a name on PATH can be a broken symlink, a wrapper that
    exits non-zero, or a script for an interpreter that is missing. Reporting it as available would
    turn a capability answer into a promise the first real call breaks.
    """
    path = shutil.which(name)
    if not path:
        return False
    try:
        completed = subprocess.run(
            [path, version_argument],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=SUBPROCESS_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return completed.returncode == 0


def probe(root):
    readable = False
    try:
        resolved = os.path.realpath(root)
        readable = os.path.isdir(resolved) and os.access(resolved, os.R_OK | os.X_OK)
    except OSError:
        readable = False

    return {
        "helperVersion": HELPER_VERSION,
        # `os.name` rather than `sys.platform`: the helper's path handling assumes POSIX semantics,
        # and that is exactly what `os.name` answers.
        "posix": os.name == "posix",
        "pythonVersion": "%d.%d.%d" % sys.version_info[:3],
        "git": tool_available("git", "--version"),
        "ripgrep": tool_available("rg", "--version"),
        "rootReadable": readable,
    }


def main():
    raw = sys.stdin.read()
    try:
        request = json.loads(raw)
    except ValueError:
        refuse("remote_helper_malformed_request")
        return

    if not isinstance(request, dict) or request.get("version") != HELPER_VERSION:
        refuse("remote_helper_version_mismatch")
        return

    root = request.get("root")
    operation = request.get("operation")
    if not isinstance(root, str) or not isinstance(operation, dict):
        refuse("remote_helper_malformed_request")
        return

    kind = operation.get("kind")
    if kind == "probe":
        respond({"version": HELPER_VERSION, "ok": True, "result": {"probe": probe(root)}})
        return

    # An unknown operation is refused rather than ignored. A helper that answered `ok` with an empty
    # result would look to a client exactly like a workspace with nothing in it.
    refuse("remote_helper_unsupported_operation")


main()
