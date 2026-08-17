## Why

Runtime review found reachable process, listener, persistence, and observability failure paths that can leave stale resources or silently misrepresent completed work. The same review found repeat-query hot paths and architecture enforcement gaps that allow concrete adapters to leak across bounded contexts despite the documented modular-monolith rules.

## What Changes

- Reap workspace and managed child processes on natural exit, abnormal drop, and partial setup failure.
- Make asynchronous terminal event subscriptions cancellation-safe and bound retained terminal replay memory.
- Preserve native-tool operation consistency when persistence fails and surface skill-evidence initialization/query failures safely.
- Batch agent and evidence queries and release shared registry locks before awaiting connector state.
- Route MCP observability and CLI delegation persistence through published ports assembled at the composition root.
- Extend architecture tests to cover infrastructure and command adapters.
- Record sanitized diagnostics when execution telemetry persistence or export fails.
- Preserve Tauri command names, request/response shapes, existing SQLite schema, and Web/mock behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-shell`: Shell processes are reclaimed after natural exit and subscriptions are cleaned up across asynchronous mount lifecycles.
- `agent-terminal-runtime`: Terminal output delivery remains responsive under burst output and retained replay memory is bounded globally.
- `skill-evolution-evidence`: Evidence initialization and feedback lookup failures are observable instead of silently presented as absent data.
- `native-runtime-architecture`: Infrastructure and command dependency rules are mechanically enforced and cross-context concrete repository access is removed.
- `agent-execution-observability`: Telemetry write failures produce sanitized diagnostics while remaining non-blocking to the primary operation.
- `runtime-performance-governance`: Registry and feedback reads use bounded batched queries and shared registry locks are not held across unrelated awaits.

## Impact

- Desktop runtime: Rust PTY/process lifecycle, SQLite repositories, MCP relay composition, native-tool operation recording, and telemetry diagnostics.
- Frontend: terminal hooks and in-memory replay buffering through the existing service boundary.
- Web runtime: no externally visible behavior change; existing service contracts remain aligned.
- No new dependencies, public command changes, or historical migration edits.
