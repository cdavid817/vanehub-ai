## Why

The standalone Plan Center and native PlanRun orchestration duplicate the planning intent already available in OnePiece sessions, while splitting one workflow across a global activity destination, separate persistence, and a second execution model. Consolidating planning into the OnePiece conversation keeps planning context, approval, and mode changes attached to the originating session and removes a large parallel execution surface.

## What Changes

- **BREAKING** Remove the Plans entry from the left workspace activity bar and remove the standalone Plan Center, draft editor, PlanRun monitoring, and Plan execution controls from both desktop and Web UI surfaces.
- Make OnePiece's session conversation bar the sole user-facing location for selecting and displaying Plan mode, with persistent text and icon semantics that distinguish read-only planning from write-capable Agent execution.
- Preserve session-scoped `plan` execution mode, its read-only tool restrictions, and the in-conversation `exit_plan_mode` approval flow; approving that request changes the session mode for a later turn and does not create a separate Plan, PlanRun, task graph, or worktree.
- **BREAKING** Remove frontend Plan service contracts, Web/mock Plan adapters, Plan polling, Plan-specific types, and associated tests and translations that exist only for standalone Plan execution.
- **BREAKING** Remove native Plan management and PlanRun orchestration commands, application/domain/infrastructure code, scheduler/driver logic, and Plan-specific persistence that are not used by session-scoped Plan mode.
- Remove or revise current main specs and active change documentation that describe the retired Plan Center, Plan draft, PlanRun, SubTask execution, and PlanRun navigation design. Archived change history remains immutable.
- Apply the consolidation to both the Tauri desktop runtime and browser Web/mock runtime while keeping React behind the existing agent-service boundary and retaining adapter parity for session chat configuration.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `main-layout-ui`: Remove the standalone Plans activity destination and make the OnePiece session conversation surface the only Plan-mode entry and status location.
- `agent-chat-configuration`: Retain read-only OnePiece Plan behavior and in-session approval while removing PlanRun transitions and navigation associations.
- `session-chat-configuration`: Constrain the persisted Plan-mode selection and effective-mode presentation to the OnePiece session conversation workflow through the shared desktop and Web/mock service contract.
- `plan-management`: Remove the standalone versioned Plan draft, approval, discovery, and execution-snapshot capability.
- `plan-execution-runtime`: Remove durable PlanRun orchestration, scheduling, attempt sessions, verification, recovery, and Run hierarchy projection.
- `agent-execution-observability`: Remove PlanRun-specific trace and telemetry contracts while retaining ordinary session and canonical Run observability.
- `frontend-runtime-architecture`: Remove the shared Plan adapter and background Plan driver contract.
- `onepiece-native-agent`: Remove structured Plan draft, attempt, discovery, repair, and orchestration credential profiles.
- `unified-todo-board`: Limit live Work Board source projection to Sessions and Scheduled Tasks.

## Impact

- Frontend: workspace routing and activity navigation, OnePiece conversation/header/composer controls, `src/plan-center`, Plan service adapters and polling, Plan types, translations, and related unit/E2E coverage.
- Native runtime: Plan commands and DTOs, Plan execution contexts and scheduler/driver code, command registration, Plan-specific database access and migrations, and associated Rust tests.
- Service boundaries: the Plan-specific frontend/native API is removed; session chat configuration remains the shared boundary used by React, with equivalent Tauri and Web/mock implementations.
- Specifications: `main-layout-ui`, `agent-chat-configuration`, and `session-chat-configuration` are revised; the standalone `plan-management` and `plan-execution-runtime` contracts are retired. Active changes that depend on those contracts must be reconciled, while archived artifacts are not edited.
- Dependencies: no replacement state-management, UI, or persistence dependency is introduced.
