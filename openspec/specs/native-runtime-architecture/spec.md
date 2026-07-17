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
