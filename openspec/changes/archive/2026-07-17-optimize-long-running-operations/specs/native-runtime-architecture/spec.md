## ADDED Requirements

### Requirement: General nonblocking native operations
The native runtime SHALL execute potentially long-running native work asynchronously so Tauri command boundaries, the Tauri main thread, and frontend rendering are not blocked by refresh, download, network, process, filesystem, Git, package, MCP, SDK, or database-heavy operations.

#### Scenario: Start potentially slow native operation
- **WHEN** the frontend requests native work that may access a remote resource, download data, spawn an external command, run package management, test a network or stdio connection, inspect Git state, create a worktree, scan many files, or perform database-heavy maintenance
- **THEN** the Tauri command SHALL return a stable operation or task id before that work completes
- **AND** the actual work SHALL continue in backend-managed asynchronous execution

#### Scenario: Keep main thread responsive
- **WHEN** a long-running native operation is running
- **THEN** it SHALL NOT block the Tauri main thread, prevent other bounded commands from responding, or freeze frontend rendering

#### Scenario: Record async operation diagnostics
- **WHEN** a long-running native operation emits progress, stdout, stderr, warnings, completion, partial completion, timeout, cancellation, or failure diagnostics
- **THEN** the native runtime SHALL associate those diagnostics with the operation or task and write redacted entries through the unified logging service

#### Scenario: Query operation status
- **WHEN** the frontend queries an in-progress or completed long-running native operation
- **THEN** the native runtime SHALL expose current status, timestamps, terminal result or error, and available logs through the service boundary

### Requirement: Bounded native request response operations
The native runtime SHALL limit direct request/response native commands to work that is bounded and not expected to depend on network latency, external process runtime, large filesystem size, download duration, or database maintenance duration.

#### Scenario: Return cached state directly
- **WHEN** a Tauri command reads cached state, validates input, or performs a small bounded persistence update
- **THEN** it MAY return the result directly without creating an operation id

#### Scenario: Reject blocking implementation for variable-duration work
- **WHEN** a new native command implementation can take variable time because of network access, process execution, large file traversal, Git operations, package management, connection testing, downloads, or database-heavy work
- **THEN** the implementation SHALL use the backend-managed operation or task model instead of waiting synchronously for completion at the Tauri command boundary
