## Context

See `proposal.md` for motivation. The current product has two separate planning paths:

- session-scoped `executionMode: "plan"`, resolved through session chat configuration and the Agent execution-policy boundary; and
- a standalone Plan vertical slice comprising the Plans workspace route, Plan Center, `PlanService`, Tauri commands, `task_orchestration` context, SQLite records, background drivers, Work Board sources, and PlanRun-owned canonical Runs.

The OnePiece composer toolbar already contains `ModeSelect` and already persists mode changes through `AgentService`. However, `useChatConfig` also queries `PlanService` for an associated PlanRun and coordinates PlanRun pauses before returning to Plan mode. The runtime separately exposes `resolve_plan_exit`, a restricted Plan tool catalog, and session execution-policy resolution; these are session safety mechanisms rather than PlanRun orchestration and must remain.

The application ships frontend and Rust runtime together, but existing databases may contain Plan history, active-looking PlanRun states, Work Board links, canonical Run ownership, and retained integration-worktree paths. Historical migration versions must remain replayable for fresh installations and upgrades.

## Goals / Non-Goals

**Goals:**

- Remove the standalone Plan vertical slice without weakening OnePiece's session-level read-only Plan enforcement.
- Make the existing OnePiece composer toolbar mode selector the single planning entry and keep its state service-backed and accessible.
- Remove all compile-time and runtime dependencies on Plan draft, PlanRun, and task orchestration from ordinary workspace, Work Board, Mission Control controls, and application bootstrap.
- Upgrade existing databases safely without deleting user project worktrees or making abandoned PlanRuns appear active.
- Keep desktop and Web/mock behavior aligned after the Plan-specific adapter contract disappears.

**Non-Goals:**

- Replacing PlanRun with another scheduler, task graph, background Agent, or autonomous execution engine.
- Changing the `inherit`, `plan`, and `execute` execution-mode data contract or the Agent policy ceiling.
- Removing the model-initiated `exit_plan_mode` approval flow, OnePiece read-only tool filtering, normal session recovery, or ordinary Run observability.
- Automatically committing, merging, deleting, or otherwise mutating a worktree previously retained by a PlanRun.
- Editing immutable artifacts under `openspec/changes/archive/`.

## Decisions

### 1. Remove the Plan vertical slice instead of adapting it behind the session UI

The frontend will remove `src/plan-center`, Plan-specific types and services, the Tauri and Web/mock Plan clients, Plan polling, Plan adapter conformance tests, and all Plan Center routing. The Rust runtime will remove the `task_orchestration` context, its commands, bootstrap assembly, driver registry, scheduler, diagnostics adapter, and command registration.

This is preferred over hiding Plan Center while retaining its service because a hidden driver would preserve the duplicated lifecycle, persistence, and failure modes that this change is intended to eliminate. It also prevents new callers from depending on a contract with no user-owned surface.

### 2. Use the existing OnePiece composer toolbar as the sole Plan-mode surface

`ModeSelect` in the session composer toolbar remains the mode control. For a `onepiece` session it continues to render the capability-oriented Plan and Agent labels, icons, and descriptions and to display the effective execution policy without relying on color. The mode selector remains disabled by the existing session admission and active-generation constraints rather than gaining a second control in the global activity bar or session header.

`useChatConfig` will stop importing `planService` and will remove `associatedPlanRun`, PlanRun polling, pause-before-plan logic, and activation callbacks. Selecting a mode will update the local `SessionExecutionMode`, and the existing debounced `AgentService.saveSessionChatConfig` path will persist and resolve it. An approved `exit_plan_mode` signal will continue to set `execute` for the following turn through the same persistence path.

Adding a second header selector was considered, but rejected because it would duplicate the composer control and create two focus, responsive-layout, and state-synchronization paths.

### 3. Preserve session planning safety code by dependency, not by name matching

Code remains when it serves session execution independently of PlanRun, including:

- `SessionExecutionMode`, chat-configuration persistence, and effective execution-policy resolution;
- the Plan-mode read-only tool catalog and server-side rejection of effectful operations;
- `exit_plan_mode`, its interactive approval state, and desktop/Web mock resolution;
- ordinary OnePiece generation, session lifecycle, cancellation, recovery, and logging.

The Plan draft generator adapter and `OnePiecePlanningPort` are removed because they produce the retired structured Plan graph even though their names mention OnePiece. Conversely, code is not removed merely because it contains the word “plan”; generic planning classifications, reconciliation plans, and unrelated domain terminology remain.

This dependency-based boundary is preferred over a broad text deletion because the latter could silently make Plan mode write-capable or remove unrelated planning concepts.

### 4. Remove every navigation and presentation path to PlanRun

The `plans` workspace destination, lazy Plan Center loader, visited/inspection state, activity-bar callback and label, `/plans` and `/plan` slash navigation, associated-Plan button, and related translations/tests will be removed. Parsing a remembered or external `/workspace/plans` path will naturally fall back to Sessions after `plans` leaves the destination allow-list.

The UI will not add a compatibility redirect to a guessed OnePiece session because there is no reliable one-to-one mapping for old global Plans. Session selection remains explicit.

### 5. Remove Plan-aware secondary projections and controls

Work Board will stop treating `plan` and `plan_run` as source kinds, stop querying Plan tables, and remove Plan summary commands and DTOs. A forward database migration will rebuild any CHECK-constrained Work Board source table as needed and remove only derived links/items whose source kind is retired.

Mission Control and generic Agent Run controls will remove PlanRun-specific pause, resume, retry, and cancellation branches. Historical canonical Runs owned by `plan_run` may remain visible as terminal historical evidence if the generic projection supports them, but they will expose no live control and will not reactivate orchestration.

This is preferred over leaving compatibility shims because those shims would require retaining `TaskOrchestrationApi` in the composition root and would keep the backend feature alive indirectly.

### 6. Retain historical Plan tables but retire all runtime access

The release will not drop Plan tables or delete Plan rows. Existing migration numbers and names remain immutable, and their schema replay will stay in the database migration layer through a self-contained legacy schema helper rather than importing the removed runtime context. Later Plan-related migration callbacks will likewise be moved or reduced to legacy migration helpers so a new database can still reach the current schema version.

A new forward migration will:

1. mark non-terminal persisted PlanRuns and PlanRun-owned canonical Runs as terminal/interrupted or cancelled using values valid for their existing schemas;
2. remove retired Plan-derived Work Board projections and rebuild constraints without `plan` and `plan_run`; and
3. leave Plan history, verification evidence, recorded worktree paths, and user filesystem worktrees intact.

No service or command will read or write the retained Plan tables after startup migration. This balances code removal with rollback safety and avoids destructive loss of historical evidence. Dropping all tables was considered, but rejected because it makes rollback and user recovery of retained-worktree provenance unnecessarily difficult.

### 7. Keep runtime boundaries explicit after contraction

React continues to use `AgentService` for session mode reads and writes; no component will call Tauri `invoke()` directly. The Tauri adapter continues mapping that shared interface to Rust session commands and SQLite-owned chat configuration. The Web/mock adapter continues deterministic per-session chat configuration and `exit_plan_mode` simulation.

The separate `PlanService` adapter family disappears entirely, so there is no replacement HTTP or mock contract to maintain. Rust remains responsible for policy resolution, tool enforcement, persistence, migrations, and unified logging.

### 8. Reconcile only mutable specification and change documentation

The change's delta specs remove the `plan-management` and `plan-execution-runtime` requirements and revise the session/UI contracts. During implementation, references in current main specs, project documentation, tests, and other unarchived changes will be updated or removed when they describe live PlanRun behavior. Archived OpenSpec changes remain untouched and discoverable through the archive index as historical evidence.

## Risks / Trade-offs

- [A PlanRun is active when the application upgrades] → Terminalize non-terminal persisted Plan and canonical Run states before runtime bootstrap, remove driver assembly, and verify no startup path can reclaim an attempt.
- [Removing the task-orchestration context breaks indirect consumers] → Treat command registries, bootstrap, Agent Run controls, Mission Control, Work Board, migrations, translations, slash commands, and tests as explicit dependency-removal checkpoints.
- [A broad deletion weakens session Plan safety] → Add focused tests for Plan tool filtering, effective read-only policy, mode persistence, and `exit_plan_mode` approval before deleting PlanRun tests.
- [Retained tables are mistaken for a live feature] → Isolate them under clearly named legacy migration code and prohibit application-layer imports or queries after migration.
- [Historical Plan worktrees become orphaned] → Preserve paths and filesystem contents; do not delete, commit, merge, or reuse them automatically.
- [Removing Plan sources deletes user-organized Work Board data] → Delete only source links/items that depend on retired Plan identities; preserve session and scheduled-task items and test migration behavior with mixed-source fixtures.
- [Old deep links or remembered routes stop resolving] → Route parsing falls back deterministically to Sessions and test both direct and recalled `/workspace/plans` paths.
- [Frontend and native commands drift during removal] → Remove the service interface, both adapters, native commands, and invoke registry entries in the same change and run contract/build checks.

## Migration Plan

1. Add and test the forward SQLite migration that terminalizes active Plan state and removes Plan-derived Work Board projections while retaining historical Plan tables and worktrees.
2. Move historical Plan schema callbacks needed by existing migration numbers into database-owned legacy helpers so migrations no longer depend on `task_orchestration`.
3. Remove native Plan commands, context, bootstrap wiring, driver/control integration, and Plan-specific secondary projections.
4. Remove PlanService, Plan Center, route/activity/slash navigation, association state, Web simulation, translations, and Plan-specific tests; retain and strengthen session Plan-mode tests.
5. Reconcile mutable specs and unarchived change documentation, then run strict OpenSpec validation and all repository validation commands, including Playwright and desktop tests because both UI behavior and Tauri startup/IPC change.

Rollback uses the preceding application version against the retained historical schema. Terminalized PlanRuns remain terminal and must not be automatically resumed after rollback; retained Plan rows and worktree paths remain available for manual inspection. If the new Work Board constraint migration has run, rollback restores application compatibility but does not recreate deleted derived Plan source links.
