## Why

Session cost grows with conversation length rather than with the work being done.

The rendered Run status observes its Run on a fixed one-second interval that never stops. One is mounted per non-user message, so a default page of fifty messages holds roughly twenty-five permanent timers, each making a Tauri IPC call into SQLite every second, and "load earlier" adds fifty more messages of them. The interval also keeps running for messages whose owner has no Run at all, because the component renders nothing in that case but never stops asking.

Streaming persistence reloads the whole message row and rewrites four columns on every flush, re-serializing the tool-use and rich-block JSON columns that a token delta never touches. Delta coalescing already bounds how often that happens, so the remaining cost is proportional to flush count times message length rather than token count times message length, but the rewrite itself is still avoidable.

The streaming row re-runs the full Markdown pipeline — remark-gfm, remark-math, rehype-katex, and rehype-highlight — on every animation frame, over the entire accumulated response. Completed rows are already memoized and unaffected.

The embedded terminals load only the fit addon, leaving xterm on its DOM renderer while CLI agents repaint full screens.

## What Changes

- Stop observing a Run once it reaches a terminal state, and stop observing an owner that has no Run and no active work, so observation cost stops growing with history length.
- Append streamed content with a SQL append instead of a read-modify-write, and leave the tool-use and rich-block columns untouched on the token path.
- Bound how often the streaming row re-renders, so Markdown re-parsing is paced rather than per-frame.
- Load the WebGL renderer for embedded terminals, falling back to the existing renderer when it is unavailable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-run-state-management`: Run status observation stops at a terminal Run and does not grow with the number of rendered history items.
- `chat-experience`: Streaming re-render and streamed-content persistence stay bounded as a response grows.

## Impact

- Shared Run status presentation component and its two call sites.
- Session message streaming persistence in Rust, including the repository's streamed-field write.
- Streaming render pacing, shared by the main window and the floating assistant.
- Terminal renderer addon; one new frontend dependency.
- The terminal-state predicate, moved beside the type it describes so both runtime adapters and the status components can reach it.
- No database migration and no service-contract change.

## Out of Scope

Four items from the same review are deliberately excluded and remain open:

- **A runtime-context assertion over the synchronous SSH and remote-shell adapters.** Investigated and rejected: the premise does not hold here. Those adapters are reached through `spawn_blocking`, and a tokio blocking-pool thread carries a runtime handle while `block_on` stays legal on it — the panic guard is a separate thread-local. An assertion on "no ambient runtime" is therefore a strictly stronger predicate than tokio's own restriction, and would fire on every remote shell operation in any debug build. `bootstrap/workspaces.rs` states the design outright: the shell sweep runs on the blocking pool because closing a shell joins a reader thread. There is no public predicate for the real invariant, and the convention an assertion would have pinned is not the one this codebase follows.

- **`synchronous=FULL` to `NORMAL`.** Raised and decided: the pragma stays. It arrived with durable session recovery and is asserted at startup, and under WAL the relaxation would only lose the last transactions to a power cut or an OS crash — but that is a durability promise this application chose to make, not a performance oversight. The streamed append below removes the write it was making expensive, which was the actual cost.
- **Message list virtualization.** Worth doing, but it changes how every message-related component test renders, because the existing virtual list measures zero height under jsdom.
- **Consolidating the remaining polling sites.** Re-measured after the Run observation fix and found not to need it. Every remaining site is already condition-gated — `refetchInterval` predicates that return `false` when idle, `enabled` flags, non-terminal guards, and in Mission Control an explicit `visibilityState` check. What is left is two effects that depend on a freshly built object and so rebuild their timer each cycle; the poll period is unchanged by that, and the re-render they were blamed for comes from writing polled data into state, which a structural comparison would not avoid while the data is genuinely changing. Recorded rather than changed.
