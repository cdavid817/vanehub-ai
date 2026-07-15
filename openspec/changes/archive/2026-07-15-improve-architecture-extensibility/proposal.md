## Why

VaneHub AI already has working React, Tauri, SQLite, MCP, SDK, and Agent surfaces, but the current implementation mixes static prototype UI, per-page data loading, hand-maintained frontend/backend contracts, ad hoc SQLite migration, and synchronous long-running operations. These gaps do not block the current build, but they will make the next wave of workspace, agent orchestration, SDK, MCP, and Web runtime features expensive and risky to extend.

## What Changes

- Introduce a frontend runtime architecture foundation for routed workspace/settings surfaces, shared data-fetching behavior, error isolation, form validation, and adapter-backed runtime services.
- Introduce a native runtime architecture foundation for app-owned storage paths, versioned SQLite migrations, structured errors/logging, long-running task tracking, and guarded external command execution.
- Introduce a contract and task foundation that keeps Rust command models and TypeScript service types synchronized, and represents SDK/MCP/Agent operations as observable tasks where needed.
- Modularize the workspace shell so demo/static data can be replaced by service-backed conversation, workflow, agent, and runtime state without growing `MainLayout` into a monolith.
- Preserve both runtime targets: the Tauri desktop runtime remains the full local-capability runtime, while the browser Web runtime remains usable through Web/mock adapters and future HTTP-backed adapters.
- No breaking user-facing feature removal is intended.

## Capabilities

### New Capabilities
- `frontend-runtime-architecture`: Shared frontend architecture requirements for routing, data fetching, error boundaries, form validation, runtime adapter selection, and workspace modularization.
- `native-runtime-architecture`: Desktop/native runtime requirements for storage, migrations, logging, long-running tasks, command safety, and security baseline behavior.
- `contract-and-task-foundation`: Cross-runtime contract requirements for generated/shared types, typed service errors, observable operations, and runtime adapter conformance.

### Modified Capabilities
- `settings-center-ui`: Settings pages should use shared data-fetching/error/form foundations instead of repeated page-local request orchestration.
- `agent-tool-registry`: Agent registry models should participate in generated/shared frontend-backend contracts.
- `agent-switching`: Agent launch and readiness flows should expose task/progress state for operations that may outlive a single immediate command.
- `interaction-modes`: Interaction lifecycle reporting should align with the native task model and structured runtime events.
- `mcp-client-management`: MCP service models and connection tests should use shared contracts, task observability, and native command safety controls.
- `sdk-dependency-management`: SDK install/update/rollback/uninstall operations should use observable task/log persistence and native runtime storage/migration foundations.

## Impact

- Frontend: `src/main-layout`, `src/settings`, `src/services`, `src/types`, UI component boundaries, and test structure.
- Backend/native: `src-tauri/src/lib.rs`, `src-tauri/src/mcp`, `src-tauri/src/sdk`, SQLite schema/migrations, command execution helpers, task/log storage, and Tauri configuration.
- Dependencies likely to be introduced: TanStack Query, React Router, Zustand, React Hook Form, Zod, React Error Boundary, a Rust-to-TypeScript type generation tool such as `specta`/`tauri-specta` or `ts-rs`, a migration tool such as `refinery`, and structured logging via `tracing`.
- Runtime boundaries: React components must continue to call service interfaces rather than Tauri `invoke()` directly; runtime-specific behavior remains behind Tauri/Web/future HTTP adapters.
