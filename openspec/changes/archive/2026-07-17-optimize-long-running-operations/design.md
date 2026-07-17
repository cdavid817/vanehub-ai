## Context

VaneHub already models some CLI refresh and package operations as asynchronous backend-managed operations, but the rule is not yet project-wide. Similar latency risks exist in MCP connection tests, SDK version discovery and package installs, downloads, remote resource access, Git inspection/worktree creation, large filesystem scans, and database-heavy maintenance paths.

The app must remain responsive in both runtimes: the Tauri desktop shell must not block the main thread, and the React UI must not freeze or discard already loaded state while service work is pending. Web/mock adapters must preserve the same contracts so browser previews and future HTTP-backed runtimes do not drift from desktop behavior.

## Goals / Non-Goals

**Goals:**

- Define a project-wide rule that potentially slow operations run asynchronously and expose observable state.
- Keep React components behind service interfaces while adapters handle desktop and Web runtime details.
- Reuse the existing native task/operation model where practical instead of adding feature-local state machines.
- Document the rule in project standards so future AI-generated code treats slow work as nonblocking by default.

**Non-Goals:**

- Replace every existing API in one step if it already returns quickly and does not risk UI or main-thread blocking.
- Add a new state management library or frontend data-fetching dependency.
- Change command safety, logging redaction, or storage ownership rules beyond applying them to asynchronous work.

## Decisions

1. Treat slow work as operation/task-backed by default.

   Native work that can exceed a short immediate response, including network access, downloads, external commands, large filesystem scans, package operations, MCP connection tests, Git operations, and database maintenance, should return an operation or task id before the work completes. This extends the existing native operation pattern instead of creating feature-specific blocking commands.

   Alternative considered: rely on frontend timeouts around synchronous commands. This still blocks the native command boundary and produces poor diagnostics when the backend is busy.

2. Preserve synchronous reads only for bounded cached state.

   Commands and services may remain request/response when they read already cached state, validate local input, or perform a small bounded write. If an implementation may touch the network, spawn a process, scan many files, download content, or perform unbounded database work, it must use async operation state.

   Alternative considered: require every command to become task-backed. That would add unnecessary polling and UI complexity for simple cached reads.

3. Keep observable state at the service boundary.

   Frontend services should expose running, success, partial success, failure, retry/refresh, and stale-data states where relevant. React components should keep previously loaded data visible during refreshes and should not call Tauri APIs directly.

   Alternative considered: let each page invent local `isLoading` behavior. That makes adapter parity and future HTTP behavior harder to verify.

4. Apply the same contract to Web/mock adapters.

   Web adapters should simulate async state and terminal outcomes for slow operations so UI behavior can be tested without desktop commands. This keeps desktop and browser behavior aligned.

   Alternative considered: Web adapters return completed results immediately. That misses loading-state regressions and can hide desktop-only responsiveness problems.

5. Update project standards as part of implementation.

   `openspec/project.md` should explicitly require asynchronous handling for potentially long-running operations and should route native diagnostics through unified logging. This makes the constraint visible to future AI coding assistants.

## Risks / Trade-offs

- Existing synchronous APIs may already be used by multiple pages -> migrate compatibility wrappers first, then update UI flows incrementally.
- More operation state can make UI code noisier -> centralize operation status mapping in services/hooks and keep components focused on rendering state.
- Background tasks can outlive their initiating view -> task status, final result, timestamps, and logs must remain queryable through the existing service boundary.
- Web/mock simulation can diverge from desktop behavior -> keep TypeScript service contracts shared and add tests for loading/running/error transitions.

## Migration Plan

1. Audit existing frontend services and native commands for network access, downloads, external commands, filesystem scans, Git, npm, MCP, and database-heavy operations.
2. Convert risky command boundaries to operation/task-backed starts with status querying while preserving existing cached reads where safe.
3. Update Tauri and Web adapters together for each affected service interface.
4. Update `openspec/project.md` with the asynchronous long-running operation standard.
5. Add tests or checks around operation start behavior, status transitions, preserved stale data, and nonblocking UI states.

## Open Questions

- What exact threshold should define "short immediate response" for implementation review? The initial guideline should be conservative: if runtime can vary with network, process execution, filesystem size, or database size, treat it as long-running.
