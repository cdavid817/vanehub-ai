# Make retained Shell startup transactional and shutdown bounded

## Why

The retained Shell subsystem currently treats registry state, replay state, runtime ownership, child processes, PTY/SSH channels, worker threads, and frontend events as if they changed atomically, but they are updated in separate steps.

For local Shell startup, the process and PTY are created before the reader, writer, worker threads, runtime retention entry, and application store entry are all established. Any intermediate failure can return early after an operating-system resource already exists. The output/exit workers can also run before the application store knows the Shell id, so a command that writes and exits immediately can lose its first output or terminal event.

Capacity enforcement is currently a count-then-open sequence. Concurrent requests can all observe remaining capacity and start resources before any one of them is registered, exceeding total or per-session limits.

Explicit close has the opposite ordering problem. Application state can be marked closed, replay/store entries can be removed, and the remote route can be discarded before the runtime confirms that the process or SSH channel has terminated. Runtime close paths ignore or weaken kill/wait failures and may execute unbounded `wait()` or thread `join()` operations. Archive, delete, idle sweep, shutdown, and route cleanup can then report success or discard ownership while a child/channel is still alive.

The result is a class of ghost-resource failures:

- the UI reports `Closed` while the process or SSH channel still exists;
- close cannot be retried because the Shell id or route was already removed;
- application shutdown or session cleanup can block indefinitely;
- concurrent creation can exceed configured limits;
- startup failure can leak a child, PTY, channel, or worker;
- fast output/exit can be lost before registration.

## What Changes

- Introduce one explicit retained-Shell lifecycle with `Opening`, `Running`, `Closing`, `Reaping`, `CloseFailed`, and terminal states. Registry, replay, route, capacity ownership, and runtime ownership remain attached to one Shell generation until termination is confirmed.
- Reserve total and per-session Shell capacity atomically before opening a local process or remote channel. The reservation is transferred to the live Shell and released only after terminal cleanup or a proven startup rollback.
- Pre-register an `Opening` Shell before workers may publish output or exit events. Runtime startup uses a launch guard that owns every acquired process, PTY, channel, stream, worker, and cleanup handoff until commit.
- Gate or buffer startup events so fast output and fast exit are retained and applied in order even when they occur during `Opening`.
- Replace boolean/implicit close behavior with a typed, idempotent close result and a staged bounded sequence. Command paths MUST NOT perform unbounded child waits, SSH waits, or worker joins.
- Add a bounded Reaper that retains ownership after a close deadline, retries without one thread per resource, records retryable/final failure, and removes store/replay/route/capacity state only after terminal confirmation.
- Publish `ShellClosed` only after terminal confirmation. Expose `Closing`, `Reaping`, and `CloseFailed` through the frontend service, Tauri adapter, Web/mock adapter, UI, and i18n without claiming that cleanup succeeded.
- Make local and remote close generation-safe. A late worker or reaper completion for an old generation cannot close, remove, or release a newer Shell that reused an identity.
- Return aggregate cleanup evidence from session archive/delete, idle sweep, and application shutdown. These flows SHALL NOT silently discard a close failure.
- Preserve shared SSH transport semantics: closing or reaping one remote Shell channel does not close unrelated channels using the same pooled connection.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `session-shell`: Define transactional startup, atomic capacity reservation, explicit intermediate lifecycle states, confirmed close, idempotent retries, aggregate session cleanup, and frontend service parity.
- `remote-terminal-runtime`: Define retained route/channel ownership, bounded remote close, generation-safe cleanup, shared-transport isolation, and remote Reaper behavior.
- `runtime-performance-governance`: Define deterministic Shell resource budgets, bounded command-path latency, bounded Reaper concurrency/queueing, and structural lifecycle metrics.

## Impact

- Affects the `workspaces` domain/application Shell model, session Shell registry/store/replay, retained local and remote runtime adapters, routed runtime, lifecycle sweeps, archive/delete/shutdown orchestration, bootstrap composition, frontend Shell services, Tauri/Web adapters, UI state, events, and desktop tests.
- May extend existing Shell DTO/state enums and close responses. Compatibility parsing SHALL tolerate previously persisted or mocked terminal states; no live pre-restart process is claimed as recoverable by this change.
- Does not require a SQLite migration unless the current implementation chooses to persist additional durable cleanup evidence. If persistence is added, the migration version MUST be selected only after scanning `main` and every active change.
- Adds platform-specific bounded termination adapters and deterministic fakes. Production code remains portable across Windows, macOS, and Linux; tests report only platforms actually executed.
- Existing Shell command names and normal open/input/resize/read behavior remain stable unless a typed response extension is required. React components continue to call the frontend service rather than Tauri directly.
- Existing natural-exit recovery work remains valid. This change specifically closes gaps in explicit startup, explicit close, capacity reservation, route retention, failure reporting, and bounded reaping.

## Non-Goals

- Persisting a live PTY or SSH channel across application restart.
- Guaranteeing termination of arbitrary detached descendants outside the process/channel ownership the platform adapter can prove.
- Redesigning SSH authentication, host-key verification, connection pooling, terminal command-boundary detection, terminal indexing, or log retention.
- Adding a general-purpose job scheduler or moving Shell lifecycle into another bounded context.
- Changing the product's configured Shell capacity values merely to make tests pass.
- Hiding cleanup failures behind unconditional success, best-effort logging, or background tasks that have no retained owner.
