## Context

The `workspaces` context owns session Shell identity, lifecycle, replay, local/remote routing, and user-visible cleanup semantics. Infrastructure adapters own portable-PTY children, SSH channels, streams, and platform termination primitives. Tauri commands and frontend adapters are transport boundaries; they must not become lifecycle coordinators.

The current implementation has two non-atomic boundaries:

1. **Startup boundary** — resources are acquired before capacity, store, replay, route, and event consumers are committed as one owned lifecycle.
2. **Close boundary** — user-visible state and lookup ownership can be removed before the external process/channel and workers are confirmed terminal.

SQLite cannot make operating-system or SSH effects transactional. This design therefore uses explicit reservations, lifecycle transitions, generation tokens, RAII launch guards, idempotent close, and a retained Reaper.

## Goals / Non-Goals

**Goals:**

- Every acquired child, PTY, SSH channel, stream, worker, route, replay buffer, and capacity permit has exactly one current owner until terminal cleanup.
- A Shell is visible as `Opening` before any worker can publish output or exit.
- Capacity admission is atomic across concurrent local and remote opens.
- Explicit close returns within a bounded command-path deadline and never reports `Closed` without terminal confirmation.
- A timed-out close transfers ownership to a bounded Reaper rather than dropping handles or blocking indefinitely.
- Repeated close, late worker completion, shutdown, sweep, archive, and delete are generation-safe and idempotent.
- Local, remote, Tauri, Web/mock, and frontend contracts expose the same lifecycle semantics.

**Non-Goals:**

- Restoring live Shell execution after application restart.
- Killing descendants that deliberately escape every process group/job/channel owned by VaneHub.
- Replacing portable-pty or the current SSH implementation.
- Introducing durable workflow orchestration for all application operations.
- Solving terminal-search pagination or log indexing.

## Invariants

1. Capacity is reserved before an external process or SSH channel is opened.
2. One capacity reservation belongs to exactly one `(shell_id, generation)` and is released exactly once.
3. A Shell store/replay entry exists in `Opening` before output or exit is accepted.
4. A launch guard owns all partially acquired startup resources until commit or cleanup handoff.
5. Runtime resources are never removed from the ownership map before they are terminal or accepted by the Reaper.
6. `ShellClosed` means terminal confirmation, not merely that close was requested.
7. `Closing`, `Reaping`, and `CloseFailed` remain addressable by Shell id and generation.
8. No command path performs an unbounded process wait, SSH wait, channel read shutdown, or thread join.
9. A late completion for generation `n` cannot mutate generation `n + 1`.
10. Repeated close is idempotent and continues the existing close/reap attempt rather than creating competing attempts.
11. Session archive/delete cannot finalize while an owned Shell is unconfirmed unless an explicit product policy records a typed deferred-cleanup outcome. This change chooses strict finalization by default.
12. Closing one remote Shell cannot close unrelated channels on a shared SSH transport.
13. Cleanup failures are represented as typed state and unified diagnostics; they are not silently discarded.

## Decisions

### 1. One generation-qualified lifecycle model

Introduce or extend the application-owned Shell aggregate/value objects:

```text
ShellGeneration(u64)

ShellLifecyclePhase:
  Opening
  Running
  Closing
  Reaping
  CloseFailed
  Exited        # terminal, natural exit
  Closed        # terminal, explicit confirmed close
  OpenFailed    # terminal, startup failed and cleanup confirmed

ShellTerminalCause:
  NaturalExit
  UserClose
  SessionCleanup
  IdleSweep
  ApplicationShutdown
  StartupRollback
  RuntimeFailure
```

`ShellGeneration` is allocated monotonically for a logical Shell identity or as an opaque unique token if identities are never intentionally reused. Every worker event, route entry, runtime handle, capacity permit, close request, and reaper completion carries both `shell_id` and `generation`.

Public DTOs MAY keep existing names for terminal compatibility, but they SHALL expose enough state to distinguish:

- close requested;
- cleanup continuing asynchronously under retained ownership;
- cleanup failed and is retryable/not retryable;
- resources confirmed terminal.

The application aggregate is the authority for legal transitions. Infrastructure cannot directly publish a terminal frontend state.

### 2. Atomic capacity reservation

Replace count-then-open with one application service/port such as:

```text
ShellCapacityController::reserve(session_id, shell_id, generation)
  -> ShellCapacityLease
```

The controller owns a single synchronization boundary covering:

- global active + reserved count;
- per-session active + reserved count;
- optional local/remote sublimits if already configured;
- reservation identity and release state.

The operation either reserves all applicable limits or reserves none. `ShellCapacityLease` is move-only/RAII in implementation terms:

- before startup commit, dropping the launch guard releases it;
- after commit, the lease moves into the retained lifecycle record;
- only terminal cleanup releases it;
- repeated release is idempotent and observable in tests.

Capacity is not released when state merely becomes `Closing`, `Reaping`, or `CloseFailed`, because the resource still consumes capacity.

### 3. Pre-register `Opening` before runtime workers publish

The application startup use case follows this order:

```text
validate request and workspace/session ownership
→ enter existing same-identity/singleflight gate
→ allocate shell id + generation
→ reserve capacity atomically
→ insert Shell aggregate/store/replay as Opening
→ reserve local/remote route identity
→ invoke retained runtime open with LaunchGuard
→ install all streams/workers/close control
→ commit runtime ownership
→ transition Opening → Running, unless an early terminal event already won
→ return typed open result
```

An `Opening` Shell is not yet writable/resizable as a normal running Shell. The frontend can render a pending state or retain current optimistic UI, but service operations return a stable `shell_not_running`/equivalent code until `Running`.

### 4. Startup launch guard and event gate

Local and remote runtime adapters use a startup guard that owns every acquired item from the first external side effect:

```text
LocalShellLaunchGuard:
  child / killer
  PTY master/slave handles
  reader/writer
  output/exit workers or worker-start permits
  cancellation/completion controls
  capacity lease handoff token

RemoteShellLaunchGuard:
  pooled transport lease (non-owning for unrelated channels)
  channel
  reader/writer
  worker controls
  route reservation
```

The guard has three terminal paths:

1. **Commit** — transfer all resources into the retained runtime entry.
2. **Rollback confirmed** — close/terminate within the startup cleanup budget, mark `OpenFailed`, remove route/replay/store, release capacity.
3. **Handoff to Reaper** — if cleanup cannot be confirmed within the startup budget, transition to `Reaping`, transfer all handles and lease to the Reaper, and keep the Shell addressable.

Workers SHALL NOT publish directly into an unknown Shell. Implement one of these equivalent patterns:

- register an event sink before worker start and gate worker start until commit-ready; or
- accept events for `Opening` into a bounded per-Shell startup buffer and drain them in sequence after runtime ownership is installed.

The buffer/gate is bounded. Overflow is a typed startup/runtime failure, not silent data loss. An early exit received while `Opening` transitions the existing aggregate to the appropriate terminal state; a later unconditional `Running` transition must compare generation and current phase and therefore cannot overwrite it.

### 5. Typed close API and state transitions

Define an application close result independent of platform-specific errors:

```text
ShellCloseDisposition:
  ClosedConfirmed
  Reaping
  CloseFailed
  AlreadyTerminal

SessionShellCloseResult:
  shell_id
  generation
  disposition
  final_state?             # only for terminal dispositions
  reason_code?
  retryable
  attempt
  cleanup_deadline_reached
```

Close use case:

```text
lookup shell + generation
→ if terminal, return AlreadyTerminal
→ if Closing/Reaping, return/reconcile existing attempt
→ if CloseFailed and retryable, atomically begin next attempt
→ transition Running/Opening → Closing
→ call retained runtime close with a bounded CloseBudget
→ on terminal confirmation: finalize terminal cleanup
→ on deadline with retained handles: transfer/confirm Reaper ownership and mark Reaping
→ on typed failure while handles remain addressable: mark CloseFailed
```

The command returns after one bounded close attempt. It does not wait for all future reaper retries. The UI receives state changes and can poll/retry through the service.

### 6. Bounded local termination

The local infrastructure adapter implements staged termination under an injected monotonic deadline:

```text
stop accepting input
→ close writer / PTY input where supported
→ bounded non-blocking exit observation
→ request graceful termination through owned platform primitive
→ bounded non-blocking exit observation
→ request forceful termination through owned platform primitive
→ bounded non-blocking reap observation
→ complete workers through cancellation/completion channels
→ join only workers already reported complete, or hand ownership to Reaper
```

Production code MUST NOT call an unbounded `child.wait()` or `JoinHandle::join()` on the command path. A dedicated platform abstraction exposes `try_wait`/bounded observation and termination outcomes. Unix and Windows implementations may differ internally, but share the application contract.

This change guarantees cleanup of the child/process primitive VaneHub owns. Process-group or Windows Job Object support MAY be used and is encouraged where already compatible, but exhaustive descendant-tree termination is not a normative guarantee of this change.

### 7. Bounded remote termination and route retention

The remote adapter performs a bounded channel-level sequence:

```text
stop input
→ request EOF/close on the Shell channel
→ cancel/read-drain workers
→ observe channel completion until deadline
→ hand the same channel and worker controls to Reaper when unconfirmed
```

The route `(shell_id, generation) -> remote runtime/channel` remains until terminal confirmation. Routed close MUST NOT unconditionally remove the route after any returned `Result`.

A pooled/shared SSH transport is not owned by one Shell. Closing/reaping one channel releases only that channel lease and must not disconnect other Shells. If the entire transport is already failed, each affected Shell receives its own generation-safe terminal/failure transition.

### 8. Retained bounded Reaper

Add one application/infrastructure collaboration with explicit resource governance:

```text
ShellReaperQueue
  bounded pending items
  bounded concurrently active attempts
  per-item attempt deadline
  bounded exponential/backoff schedule
  maximum automatic attempts
  generation-qualified completion
```

The Reaper item owns or references all handles required to continue cleanup. It is not a fire-and-forget task with no registry ownership. It uses a fixed worker pool or bounded async tasks, not one unbounded thread per Shell.

Reaper completion performs a compare-and-finalize operation:

```text
only if registry still contains same shell_id + generation
and state is Closing/Reaping/CloseFailed for the same close operation
→ mark terminal
→ remove retained runtime entry
→ remove remote route
→ finalize replay/store retention
→ release capacity lease
→ publish ShellClosed exactly once
```

If the queue is full, ownership remains in the retained runtime/registry and close returns `CloseFailed(reaper_capacity_exhausted)`; handles are not dropped. Manual retry and shutdown diagnostics remain possible.

After automatic attempts are exhausted, state remains `CloseFailed` with redacted evidence and retained ownership. A later explicit retry or application shutdown policy can attempt again.

### 9. Terminal finalization ordering

All terminal cleanup uses one application operation with this ordering:

```text
terminal confirmation from runtime/reaper
→ compare shell id/generation/current operation
→ write terminal aggregate state
→ detach retained runtime ownership
→ remove route if same generation
→ seal or release replay according to existing retention policy
→ release capacity lease
→ publish ShellClosed / terminal event
→ remove ephemeral lookup entries only when existing product semantics permit
```

If an internal store mutation fails, the system keeps enough reconciliation evidence to avoid double release and does not publish a false `ShellClosed`. In-memory operations should be designed as one lock/transaction boundary where practical.

### 10. Session archive, delete, idle sweep, and shutdown

Introduce an aggregate result:

```text
SessionShellCleanupReport:
  requested
  closed_confirmed
  already_terminal
  reaping
  failed
  entries[] { shell_id, generation, disposition, reason_code }
```

- **Archive/delete:** default to strict finalization. The session is not reported fully archived/deleted while any owned Shell remains `Closing`, `Reaping`, or `CloseFailed`. Return a typed `session_shell_cleanup_incomplete` result and preserve retryable session/Shell identity. Existing UI can show “cleanup in progress/failed” rather than claiming deletion.
- **Idle sweep:** only counts a Shell as closed after confirmation. Timed-out attempts become Reaper work and are included in sweep metrics/reporting.
- **Application shutdown:** uses a global bounded deadline. It requests close, advances the Reaper within the budget, records redacted residual resources, and allows the OS shutdown path to continue according to existing application policy. It never blocks indefinitely and never logs residual cleanup as successful.

No caller may discard close results through `let _ = ...`, unconditional route removal, or success-only event publication.

### 11. Frontend and adapter contract

The frontend Shell service adds typed lifecycle and cleanup results. Requirements:

- React components import only the service/hook layer.
- Tauri adapter maps native typed states and stable reason codes.
- Web/mock implements deterministic lifecycle transitions, capacity rejection, close/reaping/failure injection, and generation protection without spawning native resources.
- Event and pull reconciliation are both supported; a missed event cannot permanently leave the UI in the wrong state.
- New lifecycle labels, error messages, and cleanup summaries exist in every registered locale.
- Existing terminal output remains visible during `Closing`, `Reaping`, and `CloseFailed` unless current retention policy explicitly removes it after confirmed terminal cleanup.

### 12. Observability and redaction

Emit unified structured lifecycle events/metrics with bounded fields:

```text
shell_id (safe opaque id)
generation
local_or_remote
transition
close_origin
attempt
elapsed_bucket
outcome/reason_code
reaper_queue_depth
capacity totals
```

Do not log command text, terminal output, credentials, SSH secrets, unrestricted local paths, or provider payloads. Structural metrics support deterministic tests and operational diagnosis.

### 13. Initial deadlines and configurability

Exact production durations are implementation constants/configuration, not normative protocol values. They SHALL be:

- finite;
- test-injectable through a clock/budget abstraction;
- independently bounded for graceful observation, terminate observation, force/reap observation, and worker completion;
- bounded by one total command-path deadline;
- unchanged merely to make slow CI pass without evidence.

Tests use deterministic fake children/channels/workers and virtual time rather than sleeping for production deadlines.

## Failure Semantics

Use stable, non-sensitive reason codes such as:

```text
shell_capacity_exhausted
shell_open_cancelled
shell_open_setup_failed
shell_startup_cleanup_pending
shell_close_deadline_reached
shell_terminate_failed
shell_reap_deadline_reached
shell_worker_completion_pending
shell_reaper_capacity_exhausted
shell_route_stale
shell_generation_stale
session_shell_cleanup_incomplete
```

Human-readable adapter/UI messages are localized. Native error details remain in redacted unified diagnostics.

## Migration Plan

No database migration is required for the baseline design because lifecycle ownership is process-local. If implementation discovers that existing durable recovery evidence must be extended, add a new backward-compatible migration selected after scanning all active changes; do not edit released migrations.

Rollout sequence:

1. Add domain types, generation-qualified store transitions, capacity controller, and deterministic fakes behind existing APIs.
2. Add `Opening` pre-registration and local launch guard.
3. Add remote launch guard and route retention.
4. Introduce typed bounded close and Reaper while adapting existing callers.
5. Expose frontend/Web states and cleanup reports.
6. Remove legacy remove-before-close, unbounded wait/join, and ignored-result paths only after tests prove parity.

## Risks / Trade-offs

- Retaining failed resources makes failures visible and retryable but temporarily consumes capacity. This is intentional; releasing capacity while a process still exists would recreate overcommit.
- Bounded close can return `Reaping` rather than immediate `Closed`. UI and callers must handle an intermediate state.
- Strict archive/delete cleanup may delay final deletion. This is safer than losing ownership; product copy must explain the state.
- Directory/session teardown now carries richer results, increasing service DTO surface. Stable reason codes and adapter parity limit coupling.
- Platform termination primitives differ. The shared contract is terminal confirmation and bounded waiting, not identical system calls.

## Open Questions

None. Implementation MAY choose concrete type/file names that fit the current repository, but it MUST preserve the invariants, ordering, typed states, and verification requirements above.
