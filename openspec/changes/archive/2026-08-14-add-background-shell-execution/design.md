# Design

## D1. Why the baseline tool, not a OnePiece-only extended tool

The 60-second ceiling is a property of `shell`, which every native API Agent shares through `tool_catalog.rs`. Putting background execution behind the OnePiece-only registry in `native_tools/` would leave the ceiling in place for every other native Agent while creating a second, competing shell surface for OnePiece — two tools that run commands, with different limits and different permission plumbing. The permission surface does not change either way: a background command runs the same command, in the same workspace, under the same `Action::shell_exec()`/`Resource::workspace()` pair. So this extends the baseline tool.

The extended-tool registry is still the right home for capabilities that need readiness gating against an external dependency (a browser, an OCR worker, a delegated CLI). A background command has no such dependency; it needs process containment, which `platform::process` already provides.

## D2. Ownership and lifetime

A background command is owned by a **session**, not by a generation and not by a tool call. Generation scope would defeat the purpose — the first tool-use loop iteration that returns would kill the build. Session scope matches how a user thinks about it: the commands belong to the piece of work in front of them.

Three bounds, all enforced independently:

| Bound | Value | Rationale |
| --- | --- | --- |
| Concurrent commands per session | 8 | Enough for a dev server plus parallel checks; low enough that a loop cannot fork-bomb the registry. |
| Rolling output buffer per command | 256 KiB | Four times the 64 KiB single-result cap, so a caller that polls at a reasonable rate never loses output, while an unpolled chatty process cannot grow without bound. |
| Maximum lifetime | 30 minutes | Longer than any check in this repository; short enough that a forgotten process is not permanent. |

Reaching the buffer cap drops the **oldest** bytes, not the newest. For a build or test run the interesting content — the failure and the summary — is at the end. Dropping is reported explicitly so the model does not read a gap as contiguous output.

## D3. Retrieval is a cursor, not a snapshot

`shell_output` returns bytes produced since that command's previous retrieval and advances a per-command cursor. Returning the whole buffer every time would re-spend context on output the model has already read, which is the specific failure mode that makes long-running commands expensive rather than merely slow.

The cursor advances on successful retrieval only. A command that exits between two retrievals still yields its remaining bytes on the next call, together with its exit code — the exit does not discard unread output.

## D4. Process containment reuses what already exists

`ManagedChild::spawn_in` already gives piped stdio plus containment: a Windows job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, a process group leader on Unix. Both properties are exactly what background execution needs, and both are load-bearing here:

- **Process-tree termination.** Killing `npm run dev` must kill the node process it spawned, not orphan it.
- **Exit-time reaping.** The Windows job object terminates the tree when VaneHub's handle closes, so desktop exit cannot leave an unattended process even if the registry never runs its own cleanup. The Unix process group gives the equivalent guarantee through an explicit group signal during shutdown.

Because containment is inherited rather than rebuilt, the registry's own reaping path is a correctness convenience, not the only thing standing between a crash and an orphan.

## D5. No persistence, deliberately

Background command state is in-memory. A handle from a previous desktop run is rejected rather than looked up.

Persisting handles would imply either resurrecting processes across restarts (which no one asked for, and which would make a crash loop into a process leak) or storing records that can only ever resolve to "this is gone". The rejection path is the honest one, and it keeps this change out of the migration sequence entirely — no schema, no version number, no cross-worktree collision risk.

## D6. Plan mode

Plan mode excludes `shell`, so it excludes `shell_kill` for the same reason: both act on processes. It keeps `shell_output`, which observes work that was already approved before plan mode was entered and cannot start, change, or stop anything. A model that switches into plan mode mid-task can still read the build it started.

## D7. Timeout parameter

`timeout_ms` is clamped, never trusted: the schema declares a maximum and the runtime clamps again, the same shape the `file` tool's `limit` and grep's `head_limit` already use — a caller may lower a cap but never raise it above the system default. The foreground default stays 60 seconds so existing behavior is bit-for-bit unchanged when the parameter is absent.
