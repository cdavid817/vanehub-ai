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
import time

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
# How many directories one fingerprint request may cover. Bounded on both sides: the client refuses
# to send more, and this refuses to stat more, because the bound protects the machine being asked.
FINGERPRINT_PATH_LIMIT = 32
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


def sort_key(entry):
    """Directories first, then case-insensitively - the client's ordering, stated once."""
    return (0 if entry["kind"] == "directory" else 1, entry["name"].lower())


def list_directory(root, relative, after_kind_rank, after_name_key, limit):
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
    entries.sort(key=sort_key)

    # Resuming happens after the sort, because the key the client sent is the key this ordering
    # produces. Filtering first would compare against an order that does not exist yet.
    if after_name_key is not None and after_kind_rank is not None:
        resume = (after_kind_rank, after_name_key)
        entries = [entry for entry in entries if sort_key(entry) > resume]

    bound = max(1, min(limit or DIRECTORY_ENTRY_LIMIT, DIRECTORY_ENTRY_LIMIT))
    if len(entries) > bound:
        truncated = True
        entries = entries[:bound]
    return {"path": relative, "entries": entries, "truncated": truncated}, None


def directory_fingerprints(root, paths):
    """A stat per directory, and no enumeration.

    The whole point is to be cheap enough to ask on a timer. Listing each directory to decide
    whether it changed would do the expensive half of the work in order to skip the cheap half, and
    over a network that cost is paid twice.

    Every requested path gets an answer, including the ones that escape or are gone. An omitted
    entry would read to the client as "unchanged" — it compares against what it saw last time, and
    absence is not a comparison.
    """
    resolved = resolve_root(root)
    answers = []
    for relative in paths[:FINGERPRINT_PATH_LIMIT]:
        if not isinstance(relative, str):
            continue
        if resolved is None:
            answers.append({"path": relative, "state": "unreadable", "value": None})
            continue
        directory = resolve_within(resolved, relative)
        if directory is None or not os.path.isdir(directory):
            # An escape and a deletion look the same from here, and both mean the client should
            # stop expecting that directory to be where it was.
            answers.append({"path": relative, "state": "missing", "value": None})
            continue
        try:
            stat = os.stat(directory)
        except OSError:
            answers.append({"path": relative, "state": "unreadable", "value": None})
            continue
        # Nanoseconds, as an integer string. A float would compare unequal across a JSON round trip
        # on some values, which would report a change on every single poll.
        answers.append({"path": relative, "state": "known", "value": str(stat.st_mtime_ns)})
    return answers, None


# How many entries a path walk will look at before it stops and says so. A monorepo has millions
# of files and a reader is waiting on a keystroke; stopping is not the interesting part, reporting
# that it stopped is.
PATH_SCAN_LIMIT = 20000
PATH_DEPTH_LIMIT = 10
PATH_CANDIDATE_LIMIT = 2000
# The walk's own ceiling on how long it may run, in seconds. The client sends a deadline from the
# shared budget; this is what bounds a request that arrived without one, on a machine this process
# does not administer.
PATH_DEADLINE_SECONDS = 10

# Trees a reader is never trying to reach by name.
#
# Sent by the client rather than held here. This file used to carry its own copy of the list, and the
# copy fell behind: a workspace appeared to have a different shape depending on which machine it was
# on, which is the single thing a provider-neutral seam exists to prevent. The client and this script
# ship together in one binary, so there is no version to reconcile — an absent list means the client
# asked for no exclusions, not that it is too old to have any.
def normalized_exclusions(operation):
    names = operation.get("excludedDirectories")
    if not isinstance(names, list):
        return ()
    return tuple(str(name).lower() for name in names if isinstance(name, str) and name)


def exclusion_globs(excluded):
    """The same list, as ripgrep globs.

    ripgrep already applies the repository's own `.gitignore`, which is where the two sides agree by
    construction — it is Git's rules on both. These globs add the defaults on top, which ripgrep has
    no opinion about. The one place they differ from the local walk: a `.gitignore` that negates one
    of these names re-includes that tree locally and does not here, because a command-line glob
    outranks a rule file.
    """
    globs = []
    for name in excluded:
        globs.extend(["--glob", "!" + name + "/"])
    return globs


def walk_limits(operation):
    """The bounds the client sent, clamped by this script's own ceilings.

    Both, not either. The client's numbers are the shared budget and are what makes the two sides
    bound the same walk; the ceilings here are the last defence on a machine this process does not
    administer, for a request that arrived malformed or with a number somebody widened by hand.
    """
    sent = operation.get("limits")
    sent = sent if isinstance(sent, dict) else {}

    def bounded(key, ceiling):
        try:
            value = int(sent.get(key, 0) or 0)
        except (TypeError, ValueError):
            value = 0
        return ceiling if value <= 0 else min(value, ceiling)

    return {
        "max_entries": bounded("maxEntries", PATH_SCAN_LIMIT),
        "max_depth": bounded("maxDepth", PATH_DEPTH_LIMIT),
        "max_results": bounded("maxResults", PATH_CANDIDATE_LIMIT),
        "deadline_seconds": bounded("deadlineSeconds", PATH_DEADLINE_SECONDS),
    }


def search_paths(root, query, limit, excluded, limits):
    """Candidate paths for Quick Open, unranked.

    Ranking stays on the client. Scoring here would be a second implementation of an ordering the
    local provider already has, and the two would disagree first about the ties nobody writes tests
    for. What this side owns is the walk, its bounds, and the confinement — all three of which can
    only happen on the machine holding the files.

    The walk reports which bound stopped it and what it spent. One boolean cannot say whether a
    reader is looking at a short list because the tree is deep, because it is wide, or because the
    host could not read part of it — and the client used to report every one of them as an entry
    budget, which was a guess and usually the wrong one.
    """
    resolved = resolve_root(root)
    if resolved is None:
        return None, "workspace_path_escaped"
    needle = query.strip().lower().replace("\\", "/")

    entries = []
    truncated = False
    reason = None
    scanned = 0
    directories = 0
    unreadable = 0
    deepest = 0
    started = time.monotonic()
    queue = [("", 0)]
    seen = set()
    while queue:
        # Checked per directory rather than per entry: a deadline needs to be observed often enough
        # to be finite, and a clock read per filename would cost more than the walk.
        if time.monotonic() - started >= limits["deadline_seconds"]:
            truncated = True
            reason = reason or "deadline_exceeded"
            break
        relative, depth = queue.pop(0)
        directory = resolve_within(resolved, relative)
        if directory is None:
            continue
        directories += 1
        deepest = max(deepest, depth)
        try:
            with os.scandir(directory) as scan:
                children = list(scan)
        except OSError:
            # An unreadable subdirectory is a permission quirk rather than a failure, but it does
            # leave part of the workspace unexamined.
            truncated = True
            unreadable += 1
            reason = reason or "unreadable_entries"
            continue
        for entry in children:
            if entry.name.startswith("."):
                continue
            scanned += 1
            if scanned > limits["max_entries"]:
                truncated = True
                reason = "entry_budget_exhausted"
                break
            child_relative = entry.name if not relative else relative + "/" + entry.name
            target = resolve_within(resolved, child_relative)
            if target is None:
                continue
            try:
                is_directory = entry.is_dir(follow_symlinks=True)
                is_file = entry.is_file(follow_symlinks=True)
            except OSError:
                continue
            if not is_directory and not is_file:
                continue
            if is_directory:
                if entry.name.lower() in excluded:
                    continue
                if depth + 1 > limits["max_depth"]:
                    truncated = True
                    reason = reason or "depth_budget_exhausted"
                elif target not in seen:
                    seen.add(target)
                    queue.append((child_relative, depth + 1))
            if len(entries) >= limits["max_results"]:
                truncated = True
                reason = reason or "result_budget_exhausted"
                continue
            if needle and needle not in entry.name.lower() and needle not in child_relative.lower():
                continue
            entries.append(
                {
                    "name": entry.name,
                    "path": relative_to(resolved, target),
                    "kind": "directory" if is_directory else "file",
                    "size": None,
                }
            )
        if scanned > limits["max_entries"]:
            truncated = True
            reason = reason or "entry_budget_exhausted"
            break
    # The limit bounds what crosses the wire, not what the client shows: it ranks first, so cutting
    # here by anything other than the walk order would drop candidates that would have ranked well.
    bound = max(1, min(limit or limits["max_results"], limits["max_results"]))
    if len(entries) > bound:
        truncated = True
        reason = reason or "result_budget_exhausted"
        entries = entries[:bound]
    answer = {
        "entries": entries,
        "truncated": truncated,
        # Structural counters only, and no paths: this lands in a coverage a reader can see, and a
        # count is a fact about effort rather than about what the workspace contains.
        "counts": {
            "directoriesVisited": directories,
            "entriesVisited": scanned,
            "maxDepthReached": deepest,
            "unreadableEntries": unreadable,
        },
    }
    if reason is not None:
        answer["reasonCode"] = reason
    return answer, None


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
        answer["status"] = "oversized"
        answer["content"] = None
        return answer, None

    try:
        with open(path, "rb") as handle:
            raw = handle.read(FILE_BYTE_LIMIT)
    except OSError:
        return None, "workspace_path_not_found"

    try:
        answer["status"] = "text"
        answer["content"] = raw.decode("utf-8")
    except UnicodeDecodeError:
        # Decoded strictly and reported as binary rather than replaced: mojibake in a preview looks
        # like a corrupt file, and a reader cannot tell that from a file that really is corrupt.
        answer["status"] = "binary"
        answer["content"] = None
    return answer, None


def search(root, query, max_results, excluded):
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
        ]
        + exclusion_globs(excluded)
        + [
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


# How much of a matching line travels back. The same bound the local scan uses, trimmed here
# rather than on the client: sending a megabyte-long minified line so the client can cut it would
# put the cost of the bound on the wire the bound exists to protect.
SNIPPET_CHAR_LIMIT = 200


def safe_snippet(line, column_index):
    """A bounded, control-free slice of a line, centred on the match.

    Control characters are removed rather than escaped: they are not content anybody searches for,
    and an ANSI escape reaching a styled panel would be a match that repaints the interface around
    it. Centred rather than taken from the start, so a hit near the end of a long line is visible.
    """
    cleaned = "".join(
        " " if character == "\t" else character
        for character in line
        if character == "\t" or not (ord(character) < 32 or ord(character) == 127)
    )
    if len(cleaned) <= SNIPPET_CHAR_LIMIT:
        return cleaned, False
    half = SNIPPET_CHAR_LIMIT // 2
    start = min(max(column_index - half, 0), max(len(cleaned) - SNIPPET_CHAR_LIMIT, 0))
    return cleaned[start:start + SNIPPET_CHAR_LIMIT], True


def search_content(root, query, max_results, excluded):
    """Positions inside files, via ripgrep.

    ripgrep or nothing, for the same reason the path-mention search says so: a hand-rolled walk
    would be a second search with different bounds, ordering, and ignore rules, reached exactly when
    nobody could tell which one answered. `unavailable` is its own answer rather than an error,
    because a host without ripgrep is perfectly usable for everything else.
    """
    resolved = resolve_root(root)
    if resolved is None:
        return None, "workspace_path_escaped"
    if not query.strip():
        return {"matches": [], "truncated": False, "unavailable": False}, None
    limit = max(1, min(max_results or SEARCH_RESULT_LIMIT, SEARCH_RESULT_LIMIT))

    completed = run_tool(
        "rg",
        [
            "--json",
            "--fixed-strings",
            "--ignore-case",
            # One match per line, matching the local scan: a line containing the query six times is
            # one place to go, and six rows for it would push five other files off a bounded list.
            "--max-count",
            str(limit),
        ]
        + exclusion_globs(excluded)
        + [
            "--",
            query,
        ],
        resolved,
    )
    if completed is None:
        return {"matches": [], "truncated": False, "unavailable": True}, None
    if completed.returncode not in (0, 1):
        return None, "remote_search_failed"

    matches = []
    truncated = False
    for raw in completed.stdout.splitlines():
        if len(matches) >= limit:
            truncated = True
            break
        try:
            event = json.loads(raw.decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            continue
        if event.get("type") != "match":
            continue
        data = event.get("data", {})
        text = data.get("path", {}).get("text")
        line_text = data.get("lines", {}).get("text")
        submatches = data.get("submatches") or []
        if not isinstance(text, str) or not isinstance(line_text, str) or not submatches:
            # A path or line ripgrep could not encode as text. Skipped rather than guessed at: a
            # mangled path in a result list is a link that goes nowhere.
            continue
        candidate = resolve_within(
            resolved, text if not os.path.isabs(text) else os.path.relpath(text, resolved)
        )
        if candidate is None:
            continue
        byte_start = submatches[0].get("start", 0)
        column_index = len(line_text.encode("utf-8", "ignore")[:byte_start].decode("utf-8", "ignore"))
        snippet, snippet_truncated = safe_snippet(line_text.rstrip("\n"), column_index)
        matches.append(
            {
                "path": relative_to(resolved, candidate),
                "line": data.get("line_number") or 0,
                "column": column_index + 1,
                "snippet": snippet,
                "truncated": snippet_truncated,
            }
        )
    return {"matches": matches, "truncated": truncated, "unavailable": False}, None


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
        listing, error = list_directory(
            root,
            operation.get("path", ""),
            operation.get("afterKindRank"),
            operation.get("afterNameKey"),
            int(operation.get("limit", 0) or 0),
        )
        return ({"listing": listing} if listing else None), error
    if kind == "directoryFingerprints":
        paths = operation.get("paths")
        answer, error = directory_fingerprints(root, paths if isinstance(paths, list) else [])
        # `is not None`, not truthiness: asking about no directories is a valid request whose
        # answer is an empty list, and an empty list is falsy.
        return ({"fingerprints": answer} if answer is not None else None), error
    if kind == "searchPaths":
        answer, error = search_paths(
            root,
            operation.get("query", ""),
            int(operation.get("limit", 0) or 0),
            normalized_exclusions(operation),
            walk_limits(operation),
        )
        return ({"paths": answer} if answer else None), error
    if kind == "readTextFile":
        answer, error = read_text_file(root, operation.get("path", ""))
        return ({"file": answer} if answer else None), error
    if kind == "search":
        answer, error = search(
            root,
            operation.get("query", ""),
            int(operation.get("maxResults", 0) or 0),
            normalized_exclusions(operation),
        )
        return ({"search": answer} if answer else None), error
    if kind == "searchContent":
        answer, error = search_content(
            root,
            operation.get("query", ""),
            int(operation.get("maxResults", 0) or 0),
            normalized_exclusions(operation),
        )
        return ({"content": answer} if answer is not None else None), error
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
