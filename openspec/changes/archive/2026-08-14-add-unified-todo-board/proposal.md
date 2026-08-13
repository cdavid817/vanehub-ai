## Why

VaneHub AI currently exposes sessions, Plans, and scheduled tasks on separate surfaces, leaving users without one place to organize manual work alongside Agent-backed execution. A unified Todo Board is needed to turn these runtime records into a coherent, project-aware workflow without duplicating or conflating their independent lifecycle state machines.

## What Changes

- Add a global Todo Board with Inbox, Planned, In Progress, Review, and Done stages, durable ordering, priorities, project metadata, search, source filters, and a separate archive.
- Automatically reconcile existing and future top-level sessions, Plans, and scheduled tasks into board work items while allowing source-free manual work items.
- Allow one work item to link multiple sources and show live source-specific status without allowing runtime status changes to overwrite the user-controlled board stage.
- Suppress Plan attempt sessions and scheduled-task run sessions as duplicate top-level cards, presenting them as activity beneath their owning work item instead.
- Add durable source lineage and scheduled-run history needed for reconciliation, de-duplication, source navigation, and unavailable-source handling.
- Add a full-screen Todo Board destination to the workspace activity bar with accessible non-drag movement controls and responsive layouts.
- Keep all work-item operations behind runtime-neutral frontend service contracts with equivalent Tauri desktop and Web/mock behavior.

## Capabilities

### New Capabilities

- `unified-todo-board`: Defines durable work items, multi-source links, automatic reconciliation, source/runtime projections, filtering, ordering, archiving, and the unified board experience.

### Modified Capabilities

- `main-layout-ui`: Adds Todo Board as a first-class, full-screen workspace destination.
- `session-management`: Adds durable session execution lineage so user-created sessions can be distinguished from Plan and scheduled-task child sessions.
- `plan-management`: Adds globally enumerable Plan summaries and board-facing Plan aggregation across versions and runs.
- `scheduled-task-management`: Adds durable scheduled-task run history and board reconciliation behavior without changing scheduled execution semantics.

## Impact

- Both the Tauri desktop runtime and Web/mock runtime are affected.
- The frontend service boundary, runtime adapters, shared TypeScript models, React workspace navigation, localization resources, and board UI gain new contracts and behavior.
- The Rust runtime gains a work-board context, SQLite schema and migration support, source reconciliation queries, session lineage, scheduled-run history, and Tauri commands.
- Existing Session, Plan, and Scheduled Task records remain canonical; board records own organization metadata and links rather than copying runtime lifecycles.
- No alternative state-management, UI-component, database, or package dependencies are introduced.
