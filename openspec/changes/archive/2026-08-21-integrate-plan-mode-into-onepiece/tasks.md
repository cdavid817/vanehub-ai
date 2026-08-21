## 1. Protect the Retained Session Plan Mode

- [x] 1.1 Add or update frontend tests proving a `onepiece` session exposes Plan and Agent capability labels in the composer toolbar, announces the effective mode accessibly, and persists `executionMode: "plan"` through `AgentService` in both Tauri-adapter and Web/mock paths.
- [x] 1.2 Add or update Rust tests proving session Plan mode still resolves to a read-only effective policy and rejects shell, file-write, MCP-sourced, delegated, and other effectful tools.
- [x] 1.3 Preserve and verify the interactive `exit_plan_mode` contract: decline/cancel remains in Plan, approval affects only a later turn, and neither desktop nor Web/mock creates a Plan, PlanRun, task graph, or worktree.
- [x] 1.4 Record a scoped reference inventory for Plan Center, PlanService, PlanRun, task orchestration, Work Board Plan sources, Mission Control Plan controls, and historical migrations so each live dependency is removed without deleting unrelated planning terminology.

## 2. Retire Plan Persistence Safely

- [x] 2.1 Move migration callbacks required by existing Plan-related schema versions into database-owned legacy migration helpers so fresh database replay no longer imports the runtime task-orchestration context.
- [x] 2.2 Add a forward migration that terminalizes non-terminal PlanRun and PlanRun-owned canonical Run records without deleting Plan history, evidence, recorded worktree paths, or filesystem worktrees.
- [x] 2.3 Update the Work Board schema migration to remove `plan` and `plan_run` source kinds and delete only their derived links/items while preserving session and scheduled-task data.
- [x] 2.4 Add migration tests covering a fresh database, an upgraded database with active Plan state, mixed Work Board sources, idempotent migration replay, retained historical Plan rows, and no automatic worktree mutation.

## 3. Remove Native Plan Execution

- [x] 3.1 Remove Plan summary/source DTOs, queries, commands, and synchronization branches from the Work Board while retaining session and scheduled-task behavior.
- [x] 3.2 Remove PlanRun-specific pause, resume, retry, cancellation, and action projection from Agent Run controls and Mission Control, leaving historical Plan-owned Runs terminal and non-operable.
- [x] 3.3 Remove all task-orchestration Tauri commands and their invoke registrations while retaining the independent `resolve_plan_exit` command.
- [x] 3.4 Remove task-orchestration API assembly, managed state, startup driver activation, diagnostics wiring, and composition-root dependencies.
- [x] 3.5 Remove the Rust `task_orchestration` context and the structured OnePiece Plan-draft generator/port after historical migration callbacks and all consumers have been detached.
- [x] 3.6 Remove or rewrite Rust tests and fixtures that exist only for Plan drafts or PlanRun execution, and confirm remaining modules contain no application-layer query or write against legacy Plan tables.

## 4. Consolidate the Frontend into OnePiece Sessions

- [x] 4.1 Simplify `useChatConfig` by removing PlanService imports, associated-PlanRun state, polling, pause-boundary transitions, and activation callbacks while retaining session mode persistence and approved `exit_plan_mode` handling.
- [x] 4.2 Remove the Plan Center components, Plan-specific types, Plan service interface, runtime adapter selector, Tauri adapter, Web/mock adapter, polling utility, conformance tests, and Plan-only fixtures.
- [x] 4.3 Remove the `plans` workspace destination, activity-bar entry, lazy Plan Center loading, inspection/visited state, associated-run callbacks, and Plan-specific responsive/accessibility wiring.
- [x] 4.4 Update workspace route tests so direct, recalled, and unknown `/workspace/plans` locations fall back deterministically to Sessions while all remaining destinations preserve their behavior.
- [x] 4.5 Remove `/plans` and associated `/plan` slash navigation, navigation capabilities, composer open-Plan button, and corresponding command/UI tests without changing unrelated slash commands.
- [x] 4.6 Remove frontend Work Board Plan source kinds, filters, labels, and fixtures while preserving session and scheduled-task source parity with the native service.
- [x] 4.7 Remove Plan Center and PlanRun translation keys from synchronized zh-CN/en resources, retain OnePiece Plan/Agent mode copy, and update localization tests for the remaining composer toolbar.

## 5. Reconcile Mutable Documentation and Contracts

- [x] 5.1 Update mutable project documentation and current main-spec references that present Plan Center or PlanRun as live behavior, using this change's removal deltas as the source of truth and leaving `openspec/changes/archive/` untouched.
- [x] 5.2 Inspect every other unarchived change for dependencies on Plan Center, PlanService, PlanRun, or `plan-management`/`plan-execution-runtime`; revise or remove only conflicting mutable design/tasks and document when no such dependency exists.
- [x] 5.3 Update service/IPC contract snapshots and command allow-list tests to remove Plan-specific calls while retaining session chat configuration and `resolve_plan_exit` parity.
- [x] 5.4 Run `npm run docs:check`, `npm run contracts:check`, `openspec validate "integrate-plan-mode-into-onepiece" --strict`, and `openspec validate --specs --strict`, and resolve every documentation or contract failure.

## 6. Verify the Complete Refactor

- [x] 6.1 Run focused Vitest suites for workspace routing/activity navigation, OnePiece composer mode selection, chat configuration, slash commands, Work Board, and Plan-exit approval.
- [x] 6.2 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, and `npm run build` with all checks passing.
- [x] 6.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml` with all checks passing.
- [x] 6.4 Run `npx playwright test` and verify the activity bar has no Plans entry, old Plans routes fall back to Sessions, and OnePiece Plan mode remains usable in the session composer toolbar.
- [x] 6.5 Run `npm run desktop:unit:test` and `npm run test:desktop`, verify Tauri starts without task-orchestration managed state or commands, and report Windows as `PASSED`, `FAILED`, or `BLOCKED` with macOS/Linux marked `NOT RUN` unless native evidence is available.
- [x] 6.6 Run a final repository search confirming no live frontend/native code references Plan Center, PlanService, PlanRun orchestration, task orchestration, or Plan Work Board sources, while explicitly verifying retained session Plan safety and legacy migration references are the only intended matches.
