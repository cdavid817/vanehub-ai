## Why

The current IM runtime can invalidate existing chat bindings after routing changes, replace complete connector credentials with partial updates, and consume unbounded native resources through polling, per-chat task creation, and retained queue lanes. These correctness and scalability risks should be resolved before expanding connector capabilities or increasing production traffic.

## What Changes

- Preserve existing IM bindings and their persisted session configuration when global routing defaults change; apply new defaults only when creating new bindings.
- Treat connector credential edits as validated field-level patches so omitted values remain intact and failed updates cannot strand an enabled connector with incomplete credentials.
- Replace 100 ms database polling for Agent completion with exactly-once internal terminal notifications.
- Add bounded global IM execution admission in addition to the existing per-chat FIFO queue, and reclaim idle per-chat lanes.
- Serialize connector lifecycle mutations per connector and make configuration, credential, runtime replacement, testing, and rollback behavior consistent.
- Make connection tests non-disruptive to enabled inbound runtimes and ensure one connector failure does not prevent unrelated connectors from starting or stopping.
- Add safe diagnostics for ignored or malformed protocol events without logging sensitive payloads, and define intentional acknowledgement/checkpoint behavior.
- Reduce steady-state overhead through periodic deduplication maintenance, expiring access-token caches, bounded WeChat reply-context storage, and live connector status refresh.
- Deliver the work in four ordered phases: correctness fixes, event-driven completion and global limits, transactional lifecycle behavior, then secondary performance improvements.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `im-connector-management`: Strengthen routing continuity, credential patching, global admission, lane retention, lifecycle rollback, protocol diagnostics, and maintenance behavior.
- `session-runtime-management`: Require event-driven, exactly-once IM terminal completion delivery without database polling.
- `native-runtime-architecture`: Require bounded IM background work and serialized, failure-isolated connector lifecycle operations.
- `frontend-runtime-architecture`: Extend typed desktop and Web/mock IM adapter parity to lifecycle status updates and normalized mutation results.
- `settings-im-management-ui`: Preserve safe partial credential edits, use normalized routing results, and keep asynchronous connector lifecycle status current.

## Impact

- Desktop runtime: changes the Rust `communications`, `sessions`, and `agent_runtime` integration paths, connector lifecycle manager, secure credential handling, SQLite maintenance scheduling, and transport adapters.
- Frontend and Web runtime: changes the typed IM service contract, both runtime adapters, settings state synchronization, and deterministic Web/mock lifecycle behavior.
- Storage and security: no plaintext-secret fallback is introduced; any schema changes must be additive and preserve existing connector configuration, bindings, deduplication records, checkpoints, sessions, and credential references.
- Runtime boundaries: React remains isolated from Tauri APIs, and native connector protocols, credentials, routing, execution admission, and completion signaling remain behind the existing service and adapter boundaries.
- Verification: adds regression, concurrency, rollback, bounded-capacity, adapter-conformance, and performance-oriented tests while retaining the five existing connector ids and public command compatibility.
