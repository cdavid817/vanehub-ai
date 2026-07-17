# native-runtime-architecture Specification

## Purpose
Defines the native runtime foundation for app-owned storage, migrations, structured diagnostics, long-running tasks, command safety, and Tauri desktop security.
## Requirements
### Requirement: App-owned storage paths
The native runtime SHALL store application data in VaneHub-owned user data directories resolved through runtime-safe path APIs rather than relying on the process current working directory.

#### Scenario: Open desktop database
- **WHEN** the Tauri desktop runtime initializes local storage
- **THEN** it SHALL resolve the SQLite database path under an app-owned VaneHub user data directory

#### Scenario: Preserve project scope identity
- **WHEN** project-scoped data such as MCP servers is persisted
- **THEN** the native runtime SHALL store an explicit canonical project path for scope matching without using the database location as the project identity

### Requirement: Versioned SQLite migrations
The native runtime SHALL apply versioned SQLite migrations for schema creation and upgrades.

#### Scenario: Empty database startup
- **WHEN** the native runtime opens an empty database
- **THEN** it SHALL apply all required migrations in order before serving commands

#### Scenario: Existing database upgrade
- **WHEN** the native runtime opens an existing database with an older schema version
- **THEN** it SHALL apply pending migrations in order and report migration failures as structured startup errors

### Requirement: Structured native errors and logging
The native runtime SHALL use structured errors and logs for database, storage, command execution, network, validation, and task failures.

#### Scenario: Command failure
- **WHEN** a Tauri command fails
- **THEN** the native runtime SHALL return a user-displayable structured error and record a diagnostic log entry

#### Scenario: Native task diagnostic
- **WHEN** a long-running operation changes state or emits output
- **THEN** the native runtime SHALL record or emit structured task logs associated with that operation

### Requirement: Long-running native task registry
The native runtime SHALL represent long-running SDK, MCP, and Agent operations as tasks when they can exceed a short immediate command response.

#### Scenario: Start task
- **WHEN** the frontend starts a long-running native operation
- **THEN** the native runtime SHALL return a stable task id and expose task status through a service boundary

#### Scenario: Complete task
- **WHEN** a long-running native operation completes or fails
- **THEN** the native runtime SHALL expose final status, result or error, timestamps, and available logs for that task

### Requirement: Guarded external command execution
The native runtime SHALL execute external commands only through backend-owned command construction or validated user configuration without shell string interpolation.

#### Scenario: Backend-owned command
- **WHEN** the native runtime launches a known Agent or SDK command from backend-owned metadata
- **THEN** it SHALL construct the process invocation with explicit executable and argument values

#### Scenario: User-configured command
- **WHEN** the native runtime runs a user-configured MCP command
- **THEN** it SHALL validate the command configuration, avoid shell string interpolation, and record an audit log entry for the execution attempt

### Requirement: Desktop security baseline
The Tauri desktop runtime SHALL define explicit security settings for content security policy, native capabilities, and privileged runtime operations.

#### Scenario: Render packaged app
- **WHEN** the packaged desktop app loads frontend assets
- **THEN** it SHALL use an explicit CSP compatible with the app's required local functionality

#### Scenario: Privileged operation
- **WHEN** the frontend requests a privileged local operation
- **THEN** the native runtime SHALL route that request through a declared Tauri command and service adapter rather than exposing unrestricted native APIs to React components

### Requirement: SQLite-backed common settings
The native runtime SHALL persist common application settings in app-owned SQLite storage using a versioned migration.

#### Scenario: Create settings table
- **WHEN** the native runtime initializes an empty or older application database
- **THEN** it SHALL apply a migration that creates a key-value settings table before serving settings commands

#### Scenario: Load settings command
- **WHEN** the frontend requests common settings in the Tauri desktop runtime
- **THEN** the native runtime SHALL return persisted settings merged with valid default values

#### Scenario: Save setting command
- **WHEN** the frontend saves one common setting in the Tauri desktop runtime
- **THEN** the native runtime SHALL validate and upsert that setting in the SQLite settings table

### Requirement: Native Node.js environment inspection
The native runtime SHALL expose Node.js executable path and version information through a declared Tauri command.

#### Scenario: Resolve Node.js information
- **WHEN** the frontend requests Node.js environment information
- **THEN** the native runtime SHALL attempt to resolve the Node.js executable path and version without starting an interactive session

#### Scenario: Return unavailable Node.js information
- **WHEN** Node.js cannot be resolved
- **THEN** the native runtime SHALL return a user-displayable unavailable result rather than failing settings page rendering

### Requirement: Asynchronous CLI detection operations
The native runtime SHALL run CLI detection and remote version refresh as asynchronous backend-managed operations.

#### Scenario: Start CLI refresh operation
- **WHEN** the frontend requests CLI detection refresh
- **THEN** the native runtime SHALL return a stable operation id without waiting for local command checks or npm registry queries to complete

#### Scenario: Start first-run CLI refresh operation
- **WHEN** the application starts and no persisted CLI detection result exists
- **THEN** the native runtime SHALL start at most one asynchronous CLI detection refresh operation without blocking application startup

#### Scenario: CLI refresh does not block
- **WHEN** local executable checks, CLI version commands, or npm registry queries are running
- **THEN** they SHALL NOT block the Tauri main thread or frontend rendering

#### Scenario: Persist refresh results
- **WHEN** a CLI refresh operation completes or partially completes
- **THEN** the native runtime SHALL persist per-CLI status, versions, resolved path, errors, and timestamps for later cached reads

### Requirement: Asynchronous CLI package operations
The native runtime SHALL run CLI install, upgrade, and downgrade as asynchronous backend-managed operations.

#### Scenario: Start CLI package operation
- **WHEN** the frontend requests install, upgrade, or downgrade for a supported CLI and target version
- **THEN** the native runtime SHALL return a stable operation id before the npm package operation completes

#### Scenario: Capture CLI package operation logs
- **WHEN** a CLI package operation emits stdout or stderr
- **THEN** the native runtime SHALL record logs associated with the operation for display in the affected CLI card

#### Scenario: Refresh after successful package operation
- **WHEN** a CLI package operation succeeds
- **THEN** the native runtime SHALL refresh and persist the affected CLI's local detection status

### Requirement: Guarded CLI package command construction
The native runtime SHALL construct CLI package commands from backend-owned metadata rather than frontend-supplied command strings.

#### Scenario: Install selected CLI version
- **WHEN** the frontend submits an agent id and target version for a CLI package operation
- **THEN** the native runtime SHALL resolve the npm package from a backend whitelist and execute npm with explicit arguments equivalent to `npm install -g <package>@<targetVersion>`

#### Scenario: Reject unknown CLI operation target
- **WHEN** the frontend submits an unknown agent id for a CLI package operation
- **THEN** the native runtime SHALL reject the operation without executing an external command

#### Scenario: Avoid shell interpolation
- **WHEN** the native runtime executes CLI detection, npm version checks, or npm package operations
- **THEN** it SHALL construct process invocations with explicit executable and argument values and SHALL NOT rely on shell string interpolation

### Requirement: Native unified logging service
The native runtime SHALL provide a unified logging service for diagnostics and operation logs.

#### Scenario: Write diagnostic log entry
- **WHEN** a native command, storage operation, validation path, network operation, or task fails or emits diagnostics
- **THEN** the native runtime SHALL write a redacted structured log entry through the unified logging service

#### Scenario: Write operation log entry
- **WHEN** a backend-managed operation emits progress, stdout, stderr, completion, or failure output
- **THEN** the native runtime SHALL write a redacted operation log entry through the unified logging service

#### Scenario: Use configured log directory
- **WHEN** the logging service writes a log entry
- **THEN** it SHALL write under the currently configured log directory

### Requirement: Native log directory command
The native runtime SHALL expose declared Tauri commands for log directory metadata, log directory changes, and opening the active log directory.

#### Scenario: Open log directory command
- **WHEN** the frontend settings service requests to open the active log directory
- **THEN** the native runtime SHALL open the directory without exposing unrestricted filesystem APIs to React components

#### Scenario: Save log directory command
- **WHEN** the frontend settings service saves a log directory
- **THEN** the native runtime SHALL validate or create the directory before persisting the setting

### Requirement: Guarded Git project operations
The native runtime SHALL perform Git project inspection and worktree creation through backend-owned command construction and validated filesystem paths.

#### Scenario: Inspect repository with explicit Git command
- **WHEN** the native runtime inspects whether a selected folder is a Git repository
- **THEN** it SHALL construct the Git process invocation with explicit executable and argument values and SHALL NOT rely on shell string interpolation

#### Scenario: Create worktree with explicit Git command
- **WHEN** the native runtime creates a Git worktree
- **THEN** it SHALL execute `git worktree add` through explicit executable and argument values derived from validated backend-owned metadata

#### Scenario: Reject unsafe worktree name
- **WHEN** a worktree name contains path separators, `..`, control characters, or normalizes to an empty segment
- **THEN** the native runtime SHALL reject the request before executing a Git command

#### Scenario: Keep worktree outside project path
- **WHEN** a worktree target path is resolved
- **THEN** the native runtime SHALL reject the target if it is inside the selected project path

#### Scenario: Log Git diagnostics
- **WHEN** Git inspection or worktree creation fails with command output
- **THEN** the native runtime SHALL write redacted stdout, stderr, and diagnostics through the unified logging service

### Requirement: Native project persistence
The native runtime SHALL persist known project history and session project/worktree metadata in SQLite through additive migrations.

#### Scenario: Migrate known project history
- **WHEN** the native runtime initializes an empty or older database
- **THEN** it SHALL apply a migration that creates storage for known project path, display name, Git status, and last opened timestamp

#### Scenario: Migrate optional session project metadata
- **WHEN** the native runtime initializes an empty or older database
- **THEN** it SHALL apply a migration that adds optional selected project path, worktree path, worktree name, and worktree branch metadata to session storage

#### Scenario: Load older sessions
- **WHEN** an existing session has no project/worktree metadata
- **THEN** the native runtime SHALL return the session with null project/worktree metadata and its existing effective folder value

### Requirement: Nonblocking CLI command boundaries
The native runtime SHALL keep CLI refresh and CLI package Tauri command boundaries nonblocking by returning a backend-managed operation before external command work completes.

#### Scenario: Refresh command returns before detection completes
- **WHEN** the frontend requests CLI detection refresh
- **THEN** the Tauri command SHALL return a stable operation id without waiting for executable probing or npm registry commands
- **AND** timeout failures from background detection SHALL be recorded on the operation and in unified logs rather than surfacing as a Tauri command timeout

#### Scenario: Package command returns before npm completes
- **WHEN** the frontend requests CLI install, upgrade, or downgrade for a valid managed CLI and stable target version
- **THEN** the Tauri command SHALL return a stable operation id without waiting for npm install to complete
- **AND** timeout failures from the npm process SHALL be recorded on the operation and in unified logs rather than surfacing as a Tauri command timeout

### Requirement: Managed CLI package operation parity
The native runtime SHALL use one backend-owned package operation implementation for Claude Code, OpenCode, Codex CLI, and Gemini CLI.

#### Scenario: Resolve package metadata from catalog
- **WHEN** a CLI package operation starts for a managed CLI agent id
- **THEN** the native runtime SHALL resolve package name, display name, executable name, and provider from the backend CLI catalog
- **AND** it SHALL construct npm with explicit executable and argument values equivalent to `npm install -g <package>@<targetVersion>`

#### Scenario: Refresh affected CLI after package success
- **WHEN** a CLI package operation succeeds
- **THEN** the native runtime SHALL refresh and persist the affected CLI status
- **AND** the persisted status SHALL include the operation id that performed the package operation

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

### Requirement: Native usage statistics query
The native runtime SHALL expose a declared read-only Tauri command for usage statistics aggregation from SQLite chat message storage.

#### Scenario: Aggregate desktop usage statistics
- **WHEN** the Tauri adapter requests usage statistics for a supported time range
- **THEN** the native runtime SHALL aggregate `token_input` and `token_output` from persisted messages in SQLite and return the summary without exposing direct database access to the frontend

#### Scenario: Reject unsupported usage range
- **WHEN** the frontend requests an unsupported usage statistics time range
- **THEN** the native runtime SHALL reject the request with a structured user-displayable error

#### Scenario: Keep usage query bounded
- **WHEN** the native runtime handles the usage statistics command
- **THEN** it SHALL perform a bounded read-only aggregate query and SHALL NOT spawn external commands, scan the filesystem, or perform network work

### Requirement: Native CLI parameter persistence
The native runtime SHALL persist validated CLI parameter selections in a dedicated SQLite table through an additive migration and bounded repository commands.

#### Scenario: Migrate existing database
- **WHEN** the native runtime opens an empty or older VaneHub database
- **THEN** it SHALL add CLI parameter storage without deleting or rewriting existing agents, settings, sessions, messages, CLI statuses, or skills

#### Scenario: Save profile transaction
- **WHEN** the native save command receives a complete valid profile for a managed agent id
- **THEN** it SHALL validate every selection against the native catalog and commit the profile atomically

#### Scenario: Reject invalid profile transaction
- **WHEN** any submitted selection has an unknown id, wrong value type, unsupported value, reserved conflict, or control character
- **THEN** the native runtime SHALL reject the complete mutation
- **AND** it SHALL retain the previously committed profile

### Requirement: Native provider argument composition
The native runtime SHALL keep CLI parameter conversion and token placement inside provider-specific launch builders keyed by stable agent id.

#### Scenario: Compose without shell interpolation
- **WHEN** a provider invocation includes saved or per-message values
- **THEN** the native runtime SHALL pass each executable argument as a distinct process argument
- **AND** it SHALL NOT construct or execute a shell command string from those values

#### Scenario: Preserve required runtime tokens
- **WHEN** user-controlled selections are composed with a provider invocation
- **THEN** provider subcommands, structured output, session/resume, prompt delivery, and stdin tokens SHALL remain native-runtime controlled

### Requirement: Native profile diagnostics use unified logging
The native runtime SHALL report profile validation, compatibility, persistence, and provider-rejection diagnostics through unified logging with redaction.

#### Scenario: Persist profile diagnostic
- **WHEN** loading, saving, resetting, or applying a profile produces a warning or error
- **THEN** the native runtime SHALL write a redacted entry with the stable agent id and parameter id when available
- **AND** it SHALL NOT write a feature-local log file

#### Scenario: Audit launched arguments
- **WHEN** a provider process is launched with effective parameters
- **THEN** command diagnostics SHALL redact prompts, credentials, tokens, secret-like values, and sensitive runtime context before persistence

### Requirement: CLI parameter commands remain bounded
Native list, save, and reset commands SHALL perform catalog validation and small SQLite operations only and SHALL NOT probe executables, access networks, or wait for provider processes.

#### Scenario: Return profile mutation directly
- **WHEN** a valid save or reset request is handled
- **THEN** the bounded Tauri command MAY return the normalized profile directly without creating a long-running operation

