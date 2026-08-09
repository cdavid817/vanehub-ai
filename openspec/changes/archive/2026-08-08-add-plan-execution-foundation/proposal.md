## Why

VaneHub can execute manually defined Loop phases and independent Agent sessions, but it cannot turn a user goal into an approved, durable plan whose dependency-aware execution can be paused, recovered, and audited. Establishing that foundation now provides a stable scheduling and persistence contract before parallel Workers, automated integration, Guard supervision, or dynamic replanning are introduced.

## What Changes

- Add versioned Plan drafts containing ordered SubTasks, normalized dependency edges, acceptance criteria, execution limits, and validation that the graph is acyclic and bounded.
- Use the configured OnePiece provider to generate a strictly validated Plan draft, while requiring explicit user approval before execution.
- Add immutable PlanRun snapshots and durable SubTaskRun/Attempt state so an approved plan can execute in topological order with first-phase parallelism fixed to one.
- Execute each SubTask attempt in a distinct OnePiece API Agent session, pass only bounded predecessor summaries, and record verification evidence, usage, timeout, cancellation, and recovery state.
- Create and retain one isolated Plan integration worktree for the run; all first-phase SubTasks execute sequentially in that workspace, and VaneHub does not automatically merge, push, or remove it.
- Add a Plan review and execution UI through matching Tauri desktop and Web/mock service contracts, including deterministic Web/mock behavior.
- Correlate Plan, PlanRun, SubTask, Attempt, session, operation, and execution telemetry without persisting raw prompts, credentials, tool arguments, or tool results in diagnostic records.
- Keep parallel Worker worktrees, automatic result integration, Guard evaluation, mid-run DAG amendments, and target-branch application outside this change.

## Capabilities

### New Capabilities

- `plan-management`: Versioned Plan drafting, dependency validation, user editing, approval, and immutable execution snapshots.
- `plan-execution-runtime`: Durable topology-aware serial execution, SubTask attempts, bounded context transfer, verification, control, recovery, and Plan status projection.

### Modified Capabilities

- `onepiece-native-agent`: Add bounded planner and SubTask execution requests that reuse the active OnePiece Profile without copying credentials.
- `project-worktree-management`: Add guarded creation and retention of a single integration worktree for each PlanRun.
- `agent-execution-observability`: Add redacted correlation from Plan execution entities to sessions, operations, and execution runs.
- `frontend-runtime-architecture`: Add matching typed Plan service contracts for Tauri and deterministic Web/mock adapters.

## Impact

- **Desktop runtime:** Adds a `task_orchestration` bounded context, SQLite migrations and repositories, planner/executor orchestration, Tauri commands, OnePiece session integration, guarded worktree preparation, and recovery handling.
- **Web runtime:** Adds compatible in-memory/mock Plan drafting, approval, execution-state, control, and inspection behavior without claiming native Git or provider execution.
- **Frontend:** Adds shared Plan contracts, service methods, polling or subscription support, and Plan approval/execution views; React remains isolated from `invoke()` and SQLite.
- **Existing domains:** Reuses published APIs from Agent runtime, sessions, workspaces, operations, and observability. It does not reinterpret Loop lifecycle phases as SubTasks or GroupChat Seats as ephemeral Workers.
- **Dependencies and compatibility:** No alternative state library, database, UI framework, or package manager is introduced. Existing Loop and GroupChat behavior remains compatible.
