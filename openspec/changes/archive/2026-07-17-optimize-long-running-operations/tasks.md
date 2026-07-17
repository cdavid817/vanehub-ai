## 1. Audit Slow Operation Surfaces

- [x] 1.1 Audit frontend services, hooks, and pages for refresh, download, network, package, MCP, SDK, Git, filesystem, and database-backed workflows that can block rendering or replace loaded data with blank states.
- [x] 1.2 Audit Tauri commands and Rust services for remote access, external command execution, downloads, large filesystem traversal, Git operations, package management, MCP connection tests, SDK operations, and database-heavy work.
- [x] 1.3 Classify each audited operation as bounded request/response or long-running operation/task-backed, documenting any compatibility constraints.

## 2. Native Runtime Async Execution

- [x] 2.1 Convert any newly identified variable-duration Tauri command boundaries to return stable operation or task ids before background work completes.
- [x] 2.2 Ensure background native work uses backend-managed async execution and does not block the Tauri main thread.
- [x] 2.3 Ensure long-running operation progress, stdout, stderr, timeouts, partial completion, failure, and completion diagnostics are associated with the operation and persisted through unified logging.
- [x] 2.4 Ensure operation status queries expose current status, timestamps, terminal result or error, and available logs through service boundaries.

## 3. Frontend Service and Adapter State

- [x] 3.1 Update affected TypeScript service interfaces to expose loading, running, stale refresh, success, partial success, failure, retry, and terminal operation states where relevant.
- [x] 3.2 Update Tauri adapters to call declared async native operation commands through the service boundary without leaking Tauri details into React components.
- [x] 3.3 Update Web/mock adapters to simulate compatible asynchronous operation state for the same service interfaces.
- [x] 3.4 Update affected React surfaces to preserve already loaded data during refreshes and render nonblocking running/error states.

## 4. Project Standards

- [x] 4.1 Add an asynchronous long-running operation section to `openspec/project.md` covering refresh, download, network resource access, package operations, external command execution, connection testing, filesystem scanning, Git operations, and database-heavy work.
- [x] 4.2 State in `openspec/project.md` that future AI-generated code must use service boundaries, runtime adapters, backend-managed operations/tasks, and unified logging for potentially slow work.

## 5. Verification

- [x] 5.1 Add or update frontend tests for loading/running/error transitions and stale-data preservation on affected pages.
- [x] 5.2 Add or update Rust tests for operation start behavior, nonblocking command boundaries, and status/log retrieval where native code changes.
- [x] 5.3 Run `npm run test`.
- [x] 5.4 Run `npm run build`.
- [x] 5.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.6 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 5.7 Run `openspec validate --specs --strict`.
- [x] 5.8 Run `openspec validate "optimize-long-running-operations" --strict`.
