## Why

An audit of single-Agent session creation and the Traces panel at `main@d6e1d6ff` found nine defects that make the product state wrong rather than merely unpolished: creation can be blocked by a hidden field the user cannot see, a directory inspection result can be attributed to the wrong path, a created session can leave its operation permanently unfinished, and the execution timeline reports unknown measurements as "still running", loses the reader's page and selection on every live refresh, lets a late start event reopen a terminal Run, and silently truncates events at 5000 without saying so.

Four of these violate requirements this repository already states. `Waterfall-ready bounded timeline projection` requires truncation and coverage metadata when a run exceeds the bound, and the timeline query returns none. The same requirement distinguishes a duration that is *running* from one that is *unavailable*, and both the waterfall and its accessible label collapse them into "still running". `Structured execution span classification` requires a kind derived from native attributes for a process span whose name does not say so, and the terminal producer emits four spans carrying no kind-bearing attribute at all. Correctness that is specified but not enforced is worse than an unstated rule, because every reader downstream is entitled to rely on it.

## What Changes

- Derive the create-session submit guard and the enable guard from one normalized, workspace-mode-aware validation result, so a draft belonging to an inactive workspace mode cannot block submission through a control the user cannot reach.
- Bind each project inspection result to the request that asked for it, so a late-returning inspection for an abandoned path cannot replace the current path's result or its derived Git capability.
- Give session creation an explicit consistency boundary between persisting the session and completing its workspace operation, and make an already-persisted session's completion recoverable and idempotent rather than silently dropped.
- Split the frontend creation flow into distinguishable states (creating, operation succeeded, resolving session, resolved, recoverable error) so a transient canonical-session read failure retries the read instead of stranding the dialog or inviting a duplicate session.
- Require VaneHub's own span producers to declare their structured kind, and reconcile the unclassified token so the specification and the implementation name the same value.
- Model span lifecycle and measurement quality as two independent facts in the timeline projection and in every surface that renders it, so a terminated span with an unusable timestamp reads as "ended, duration unknown" rather than "still running".
- Key timeline and run-list queries on data identity alone and invalidate them on transitions, so a live refresh preserves loaded pages, the reader's selection, and open detail and filter state.
- Make execution status transitions monotonic on every write path, so a replayed or late start can idempotently complete metadata but cannot return a terminal Run or span to running.
- Return verifiable coverage with a bounded timeline and page events independently, so a consumer can tell a run with no events from a response that stopped at the limit.

Affects both runtimes: the desktop runtime owns the native producer, storage, and query changes, and the Web/mock adapter MUST expose the same coverage, kind, and duration-quality contract so the panel behaves identically without native side effects. No new Tauri command is introduced; the timeline response gains coverage fields, so both the Tauri adapter and the Web/mock adapter change together at the existing service boundary. No component gains a direct `invoke()` call.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-execution-observability`: VaneHub-owned span producers MUST declare a structured kind rather than relying on inference that cannot succeed; the unclassified kind token becomes one value named identically in spec and DTO; span lifecycle state and duration-measurement quality become separately reported facts that no consumer may collapse; execution status transitions become monotonic across start, finish, and replay on every write path; and a bounded timeline MUST report event coverage and support independent event paging instead of a silent limit.
- `session-management`: Session creation submission and its enablement guard MUST derive from one workspace-mode-aware validation result; project inspection results MUST be attributed to the request and path that produced them; session persistence and workspace-operation completion MUST have a declared consistency boundary with a recoverable, idempotent completion path; and a successful operation whose canonical session read fails MUST remain resolvable without creating a second session.

## Impact

- Native: `execution_observability` span-kind declaration, timeline query coverage and event paging, and monotonic status transitions in the SQLite repository; `agent_runtime` terminal span producer attributes; `sessions` creation completion consistency and recovery.
- Frontend: create-session dialog validation, inspection request attribution, and creation-state modeling; Traces panel query keys and invalidation; waterfall bar placement, span detail, and accessible label duration semantics.
- Service boundary: the execution timeline DTO gains coverage metadata and event paging, changing `tauri-execution-observability-client` and the Web/mock client together; frontend service interfaces stay the only dependency React has.
- Persistence: no schema migration is required for status monotonicity, which is a write-path conflict-resolution change; event paging reuses the existing `execution_events` ordering.
- Verification: existing trace and create-session suites pass today and do not detect any of these defects, so this change MUST add the regression cases that fail before the fix.
