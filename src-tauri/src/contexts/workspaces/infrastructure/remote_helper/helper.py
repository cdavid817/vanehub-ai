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
#
# Confinement happens here rather than on the client, and it has to: only this machine can tell a
# symlink from a directory, and a client that checked a path it was told about would be checking a
# claim rather than a fact.

import base64
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

# The same bounds the local provider uses, so a workspace does not change size when it moves to a
# remote host. A reader comparing two sessions would otherwise see a difference that is about the
# transport rather than about the work.
DIRECTORY_ENTRY_LIMIT = 500
FILE_BYTE_LIMIT = 1024 * 1024
SEARCH_RESULT_LIMIT = 200
GIT_OUTPUT_LIMIT = 2 * 1024 * 1024


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


def succeed(result):
    respond({"version": HELPER_VERSION, "ok": True, "result": result})


def resolve_root(root):
    """The real root, or nothing.

    `realpath` first, so every later comparison is between resolved paths. Comparing a candidate's
    real path against an unresolved root would let a symlinked root make every child look like an
    escape, and comparing unresolved against unresolved would let a symlinked child escape.
    """
    try:
        resolved = os.path.realpath(root)
    except OSError:
        return None
    return resolved if os.path.isdir(resolved) else None


def resolve_within(root, relative):
    """A real path inside the root, or nothing.

    Two refusals before the filesystem is touched — an absolute path and a `..` component — and one
    after: the resolved candidate must be the root or sit under `root + separator`. The separator
    matters. Without it `/work/app-secrets` passes a `startswith("/work/app")` test, which is a real
    escape that reads like a typo.
    """
    if relative in ("", "."):
        return root
    if os.path.isabs(relative) or relative.startswith("~"):
        return None
    parts = relative.replace("\\", "/").split("/")
    if any(part in ("..", "") for part in parts):
        return None
    candidate = os.path.realpath(os.path.join(root, *parts))
    if candidate == root or candidate.startswith(root + os.sep):
        return candidate
    return None


def relative_to(root, path):
    if path == root:
        return ""
    return os.path.relpath(path, root).replace(os.sep, "/")


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


def run_tool(name, arguments, cwd):
    """One subprocess, as an argument array, with a pinned locale.

    No shell: an argument array is the difference between a path with a space in it and a path that
    executes. `LC_ALL=C` pins git's message language, because the client classifies some outcomes by
    matching output text and a translated message would be classified as an unknown failure.
    """
    path = shutil.which(name)
    if not path:
        return None
    environment = dict(os.environ)
    environment["LC_ALL"] = "C"
    environment["GIT_TERMINAL_PROMPT"] = "0"
    try:
        return subprocess.run(
            [path] + arguments,
            cwd=cwd,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=SUBPROCESS_TIMEOUT_SECONDS,
            env=environment,
        )
    except (OSError, subprocess.SubprocessError):
        return None


def probe(root):
    resolved = resolve_root(root)
    readable = bool(resolved) and os.access(resolved, os.R_OK | os.X_OK)
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


def list_directory(root, relative):
    directory = resolve_within(root, relative)
    if directory is None:
        return None, "workspace_path_escaped"
    if not os.path.isdir(directory):
        return None, "workspace_path_not_found"

    entries = []
    truncated = False
    try:
        with os.scandir(directory) as scan:
            for entry in scan:
                if entry.name.startswith("."):
                    continue
                # Resolved per entry, so a symlink pointing outside the root is skipped rather than
                # listed under a name that suggests it is inside.
                target = resolve_within(root, relative_to(root, os.path.join(directory, entry.name)))
                if target is None:
                    continue
                try:
                    is_directory = entry.is_dir(follow_symlinks=True)
                    is_file = entry.is_file(follow_symlinks=True)
                except OSError:
                    continue
                if not is_directory and not is_file:
                    # A socket, a device, or a fifo. Naming it would offer a reader something the
                    # panel cannot open.
                    continue
                size = None
                if is_file:
                    try:
                        size = entry.stat(follow_symlinks=True).st_size
                    except OSError:
                        continue
                entries.append(
                    {
                        "name": entry.name,
                        "path": relative_to(root, target),
                        "kind": "directory" if is_directory else "file",
                        "size": size,
                    }
                )
    except OSError:
        return None, "workspace_path_not_found"

    # Directories first, then case-insensitive by name: the same order the local provider produces,
    # so the same workspace does not reorder itself when it moves to a remote host.
    entries.sort(key=lambda item: (0 if item["kind"] == "directory" else 1, item["name"].lower()))
    if len(entries) > DIRECTORY_ENTRY_LIMIT:
        truncated = True
        entries = entries[:DIRECTORY_ENTRY_LIMIT]
    return {"path": relative, "entries": entries, "truncated": truncated}, None


def read_text_file(root, relative):
    path = resolve_within(root, relative)
    if path is None:
        return None, "workspace_path_escaped"
    if not os.path.isfile(path):
        return None, "workspace_path_not_found"

    try:
        size = os.path.getsize(path)
    except OSError:
        return None, "workspace_path_not_found"

    name = os.path.basename(path)
    answer = {"path": relative_to(root, path), "name": name, "size": size}
    if size > FILE_BYTE_LIMIT:
        # A fact about the file, not a failure. A reader who asked for a 4 GiB core dump needs to be
        # told why there is no preview rather than shown an error.
        answer["status"] = "too-large"
        answer["content"] = None
        return answer, None

    try:
        with open(path, "rb") as handle:
            raw = handle.read(FILE_BYTE_LIMIT)
    except OSError:
        return None, "workspace_path_not_found"

    try:
        answer["status"] = "available"
        answer["content"] = raw.decode("utf-8")
    except UnicodeDecodeError:
        # Decoded strictly and reported as binary rather than replaced: mojibake in a preview looks
        # like a corrupt file, and a reader cannot tell that from a file that really is corrupt.
        answer["status"] = "binary"
        answer["content"] = None
    return answer, None


def search(root, query, max_results):
    resolved = resolve_root(root)
    if resolved is None:
        return None, "workspace_path_escaped"
    limit = max(1, min(max_results, SEARCH_RESULT_LIMIT))

    # ripgrep or nothing. A hand-rolled walk would be a second search with different bounds,
    # ordering, and ignore rules, reached exactly when nobody could tell which one answered.
    completed = run_tool(
        "rg",
        [
            "--json",
            "--files-with-matches",
            "--max-count",
            "1",
            "--fixed-strings",
            "--",
            query,
        ],
        resolved,
    )
    if completed is None:
        return None, "remote_ripgrep_missing"
    # 1 is "no matches", which is an answer rather than a failure. Anything else is not.
    if completed.returncode not in (0, 1):
        return None, "remote_search_failed"

    matches = []
    truncated = False
    for line in completed.stdout.splitlines():
        if len(matches) >= limit:
            truncated = True
            break
        try:
            event = json.loads(line.decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            continue
        if event.get("type") != "begin":
            continue
        text = event.get("data", {}).get("path", {}).get("text")
        if not isinstance(text, str):
            # A path ripgrep could not encode as text. Skipped rather than guessed at: a mangled
            # path in a result list is a link that goes nowhere.
            continue
        candidate = resolve_within(root, text if not os.path.isabs(text) else os.path.relpath(text, resolved))
        if candidate is None:
            continue
        matches.append(
            {
                "name": os.path.basename(candidate),
                "path": relative_to(root, candidate),
                "kind": "file",
                "size": None,
            }
        )
    return {"matches": matches, "truncated": truncated}, None


def git_command(root, arguments):
    resolved = resolve_root(root)
    if resolved is None:
        return None, "workspace_path_escaped"
    completed = run_tool("git", arguments, resolved)
    if completed is None:
        return None, "remote_git_missing"
    if completed.returncode != 0:
        stderr = completed.stderr.decode("utf-8", "replace").lower()
        if "not a git repository" in stderr:
            # Not a repository is an answer: the panel shows "no version control here" rather than
            # an error, and the distinction is the whole reason the locale is pinned.
            return {"isRepository": False, "stdoutBase64": None, "truncated": False}, None
        return None, "remote_git_failed"

    stdout = completed.stdout
    truncated = len(stdout) > GIT_OUTPUT_LIMIT
    if truncated:
        # Cut here and say so. Parsing a half-diff on the client would render as a smaller change,
        # which is the one way a diff can be wrong without looking wrong.
        stdout = stdout[:GIT_OUTPUT_LIMIT]
    return (
        {
            "isRepository": True,
            "stdoutBase64": base64.b64encode(stdout).decode("ascii"),
            "truncated": truncated,
        },
        None,
    )


def git_status(root):
    return git_command(
        root,
        [
            "-c",
            "core.quotepath=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
    )


def git_diff(root, relative, staged):
    if resolve_within(root, relative) is None:
        return None, "workspace_path_escaped"
    arguments = [
        "-c",
        "core.quotepath=false",
        "diff",
        "--no-ext-diff",
        "--no-color",
        "--unified=3",
    ]
    if staged:
        arguments.append("--cached")
    # `--` before the path, so a file named like an option is a file.
    arguments.extend(["--", relative])
    return git_command(root, arguments)


def dispatch(root, operation):
    kind = operation.get("kind")
    if kind == "probe":
        return {"probe": probe(root)}, None
    if kind == "listDirectory":
        listing, error = list_directory(root, operation.get("path", ""))
        return ({"listing": listing} if listing else None), error
    if kind == "readTextFile":
        answer, error = read_text_file(root, operation.get("path", ""))
        return ({"file": answer} if answer else None), error
    if kind == "search":
        answer, error = search(root, operation.get("query", ""), int(operation.get("maxResults", 0) or 0))
        return ({"search": answer} if answer else None), error
    if kind == "gitStatus":
        answer, error = git_status(root)
        return ({"git": answer} if answer else None), error
    if kind == "gitDiff":
        answer, error = git_diff(root, operation.get("path", ""), bool(operation.get("staged")))
        return ({"git": answer} if answer else None), error
    # An unknown operation is refused rather than ignored. A helper that answered `ok` with an empty
    # result would look to a client exactly like a workspace with nothing in it.
    return None, "remote_helper_unsupported_operation"


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

    result, error = dispatch(root, operation)
    if error:
        refuse(error)
        return
    succeed(result)


main()
