## Why

Some refresh, download, network lookup, filesystem, database, and external-command paths can take long enough to freeze the desktop shell or make the frontend appear unresponsive if they are handled synchronously. VaneHub needs a project-wide contract that treats potentially slow operations as asynchronous work by default, so future AI-generated code preserves UI responsiveness.

## What Changes

- Require potentially long-running frontend service calls to expose loading, running, retry, refresh, success, failure, and stale-data states without blocking React rendering.
- Require Tauri commands that start slow native work to return quickly with a stable operation or task id, while the actual work runs in backend-managed async execution.
- Require remote resource access, downloads, package operations, CLI checks, MCP connection tests, Git inspections/worktree operations, and heavy database/file operations to avoid blocking the Tauri main thread or frontend event loop.
- Require Web/mock adapters to preserve the same asynchronous service contracts with simulated operation states.
- Add project standard language requiring future code generation to handle potentially time-consuming operations asynchronously.
- No breaking changes to existing service boundaries are intended; existing synchronous APIs may be migrated behind compatible operation/status models where needed.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `frontend-runtime-architecture`: Add a frontend-wide responsiveness contract for service-backed long-running operations and runtime adapter parity.
- `native-runtime-architecture`: Expand native async operation requirements beyond CLI-specific paths to all potentially slow native operations.

## Impact

- Affects both Tauri desktop runtime and browser Web/mock runtime.
- Affects frontend services, runtime adapters, loading/error UI state, native Tauri command boundaries, operation/task registry usage, unified logging, and project standards in `openspec/project.md`.
- Preserves the existing architecture rule that React components use service interfaces instead of direct Tauri `invoke()` calls.
