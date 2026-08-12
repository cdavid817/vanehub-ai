## Why

Plan execution currently occupies a top-level product surface while providing an incomplete, manually stepped orchestration workflow that is outside the product's core multi-agent session-management focus. Removing it reduces navigation and runtime complexity and eliminates a large task-orchestration subsystem that no longer has a supported product surface.

## What Changes

- **BREAKING** Remove the Plan entry from the desktop/Web activity bar and remove the Plan Center UI, frontend contracts, Tauri adapter, Web/mock adapter, polling helper, Plan types, translations, and tests.
- **BREAKING** Remove all Plan Tauri commands and command registration, the `task_orchestration` bounded context, OnePiece planner/executor/verifier integration, Plan recovery, Plan diagnostics, and runtime assembly.
- Remove Plan-specific OnePiece orchestration and execution-observability contracts while leaving ordinary Agent sessions, chat Plan Mode, Loop execution, GroupChat, and scheduled tasks unchanged.
- Remove Plan-specific worktree creation APIs while preserving the general worktree and Loop worktree capabilities.
- Preserve already-shipped SQLite migration ordering and compatibility: existing Plan tables may remain as inert legacy schema so upgrades do not rewrite migration history or destroy user data.
- Keep archived OpenSpec changes immutable; update only current main specifications through this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plan-management`: Remove the Plan draft, versioning, planner-generation, validation, and approval capability.
- `plan-execution-runtime`: Remove durable PlanRun scheduling, attempt execution, verification, controls, and recovery.
- `project-worktree-management`: Remove PlanRun-specific integration-worktree creation, ownership, and retention behavior.
- `frontend-runtime-architecture`: Remove the cross-runtime Plan service/adapter and Plan Center projection contracts.
- `onepiece-native-agent`: Remove OnePiece Plan generation and Plan SubTask execution integration.
- `agent-execution-observability`: Remove PlanRun/SubTask/Attempt-specific correlation and summary behavior.

## Impact

- Desktop and Web/mock runtimes both lose the Plan product surface and its frontend adapter contract.
- Tauri command registration, bootstrap assembly, Rust context modules, frontend navigation, translations, types, and Plan-specific tests are removed.
- The existing agent-runtime, sessions, operations, workspaces, unified logging, and observability boundaries remain; only their Plan-specific consumers or published APIs are removed.
- Existing databases retain legacy Plan tables through the historical additive migration, but no live code reads or writes those tables after this change.
