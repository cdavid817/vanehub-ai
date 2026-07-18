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
The native runtime SHALL perform bounded all-tool and targeted CLI installation discovery and version refresh as asynchronous backend-managed operations.

#### Scenario: Start first CLI detection
- **WHEN** the application starts and no persisted CLI detection result exists
- **THEN** the native runtime SHALL start at most one asynchronous all-tool CLI detection refresh operation without blocking application startup

#### Scenario: Start targeted CLI detection
- **WHEN** the frontend requests refresh for a supported stable agent id
- **THEN** the native runtime SHALL return an operation id before bounded path enumeration, version probes, or registry queries complete

#### Scenario: CLI refresh does not block
- **WHEN** local executable checks, CLI version commands, or npm registry queries are running
- **THEN** they SHALL NOT block the Tauri main thread or frontend rendering

#### Scenario: Persist refresh results
- **WHEN** a CLI refresh operation completes or partially completes
- **THEN** the native runtime SHALL persist per-CLI status, active path, bounded installation distribution, versions, conflict state, errors, and timestamps for later cached reads

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
The native runtime SHALL construct CLI package commands and lifecycle eligibility from backend-owned metadata and the freshly validated active installation rather than frontend-supplied command strings.

#### Scenario: Install selected CLI version
- **WHEN** the frontend submits a supported agent id and stable target version for an eligible missing or npm-managed CLI
- **THEN** the native runtime SHALL resolve the npm package from a backend whitelist and execute npm with explicit arguments equivalent to `npm install -g <package>@<targetVersion>`

#### Scenario: Reject unsafe active source
- **WHEN** the active executable is non-npm, unknown, broken, or no longer matches the confirmed lifecycle plan
- **THEN** the native runtime SHALL reject automatic npm mutation for that active installation and return concise manual or source-native guidance

#### Scenario: Reject unknown CLI operation target
- **WHEN** the frontend submits an unknown agent id for a CLI package operation
- **THEN** the native runtime SHALL reject the operation without executing an external command

#### Scenario: Avoid shell interpolation
- **WHEN** the native runtime executes CLI detection, npm version checks, or npm package operations
- **THEN** it SHALL construct process invocations with explicit executable and argument values and SHALL NOT rely on shell string interpolation

### Requirement: Bounded CLI installation enumeration
The native runtime SHALL enumerate supported CLI installations from backend-owned bounded candidates and SHALL NOT recursively scan arbitrary user disks.

#### Scenario: Enumerate PATH and known locations
- **WHEN** the native runtime detects a supported CLI
- **THEN** it SHALL inspect all PATH results and a bounded platform-specific set of known locations, normalize candidates, and probe distinct targets with timeouts

#### Scenario: Preserve active PATH entry
- **WHEN** one or more PATH results exist
- **THEN** the native runtime SHALL identify the first valid PATH result as the active installation while retaining other distinct installations for diagnostics

#### Scenario: Executable is installed but broken
- **WHEN** a candidate executable exists but its bounded version probe exits unsuccessfully or times out
- **THEN** the native runtime SHALL preserve it as installed but non-runnable and record redacted diagnostics through unified logging

### Requirement: Serialized CLI package mutations
The native runtime SHALL prevent overlapping managed CLI package mutations.

#### Scenario: Package mutation already running
- **WHEN** an install, upgrade, or downgrade is requested while another managed CLI package mutation is queued or running
- **THEN** the native runtime SHALL reject or queue the new mutation deterministically without launching concurrent global package-manager writes

#### Scenario: Detection during package mutation
- **WHEN** a safe read-only detection request occurs while a package mutation is running
- **THEN** the runtime MAY execute or defer detection but SHALL keep the Tauri command boundary nonblocking and SHALL NOT corrupt the package mutation

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
The native runtime SHALL expose a declared read-only Tauri command that aggregates normalized SQLite usage records without exposing direct database access to the frontend.

#### Scenario: Aggregate desktop usage statistics
- **WHEN** the Tauri adapter requests usage statistics for a supported time range
- **THEN** the native runtime SHALL return separated reported-token and estimated-character totals, coverage, counted sessions and responses, local-calendar daily trend points, and per-Agent rows
- **AND** it SHALL key Agent rows by stable Agent id rather than matching display names

#### Scenario: Reject unsupported usage range
- **WHEN** the frontend requests an unsupported usage statistics time range
- **THEN** the native runtime SHALL reject the request with a structured user-displayable error

#### Scenario: Keep usage query bounded
- **WHEN** the native runtime handles the usage statistics command
- **THEN** it SHALL perform indexed bounded read-only aggregate queries
- **AND** it SHALL NOT spawn external commands, scan the filesystem, access the network, or load prompt and response bodies for aggregation

#### Scenario: Use desktop-local calendar semantics
- **WHEN** the native runtime filters or groups a bounded usage range
- **THEN** it SHALL derive range boundaries and daily bucket dates from the desktop user's local calendar rather than UTC midnight

### Requirement: Native normalized usage persistence
The native runtime SHALL persist versioned normalized usage records in SQLite through the session runtime and database layer.

#### Scenario: Enforce one record per response
- **WHEN** the native runtime writes usage for an assistant message
- **THEN** SQLite SHALL enforce at most one usage record for that message
- **AND** session or message deletion SHALL remove the owned usage record through the ownership relationship

#### Scenario: Enforce accounting invariants
- **WHEN** a usage record is inserted or updated
- **THEN** token and character counts SHALL be non-negative
- **AND** reported accounting SHALL use token units while estimated accounting SHALL use character units

#### Scenario: Protect usage-record privacy
- **WHEN** usage accounting is persisted
- **THEN** the usage record SHALL NOT contain prompt text, response text, raw CLI events, credentials, or secret values

#### Scenario: Index monitoring dimensions
- **WHEN** the usage-record migration completes
- **THEN** occurrence-time and stable-Agent-id query dimensions SHALL have indexes suitable for bounded trend and Agent aggregation

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

### Requirement: Native IM domain ownership
The Rust native layer SHALL own connector protocols, secure credential access, SQLite connector state, background lifecycle, external-chat routing, and integration with Agent execution.

#### Scenario: Start native runtime
- **WHEN** the Tauri application completes database migration and native setup
- **THEN** it SHALL initialize the IM runtime manager and asynchronously start eligible enabled connectors without blocking window creation

### Requirement: Non-blocking IM commands
Variable-duration IM operations SHALL not block the Tauri command thread or frontend settings shell.

#### Scenario: Start or test connector
- **WHEN** the frontend requests connector start, restart, stop, test, or authorization polling
- **THEN** the native command SHALL schedule or await bounded asynchronous work and SHALL keep unrelated connector and settings operations responsive

### Requirement: Shared native chat entry point
The native runtime SHALL expose Agent message execution as an internal service rather than coupling it exclusively to a Tauri command.

#### Scenario: Execute from command and router
- **WHEN** desktop chat or the IM router submits a message
- **THEN** both callers SHALL use the same internal validation, persistence, process launch, parsing, lifecycle, and completion implementation

### Requirement: Native connector storage migration
The native runtime SHALL apply additive SQLite migrations for connector configuration, routing, credential references, bindings, deduplication, and checkpoints.

#### Scenario: Upgrade existing database
- **WHEN** an existing VaneHub database is opened after the IM feature is installed
- **THEN** the migration SHALL preserve all existing settings, agents, sessions, messages, projects, Skills, SDK data, and MCP data

### Requirement: Testable native boundaries
Platform transports and credential storage SHALL be replaceable with deterministic test doubles within Rust tests.

#### Scenario: Run connector tests without credentials
- **WHEN** native unit and integration tests execute
- **THEN** they SHALL validate normalization, deduplication, queueing, binding, status, retries, secure-store calls, and final delivery without contacting live IM services or the real OS credential store

### Requirement: Native session maintenance jobs
The desktop runtime SHALL run session maintenance jobs in Rust after database and unified logging initialization.

#### Scenario: Start maintenance jobs
- **WHEN** the desktop runtime initializes successfully
- **THEN** it SHALL run startup recovery and automatic archival checks without blocking the main window UI

#### Scenario: Hourly automatic archival schedule
- **WHEN** automatic archival is enabled
- **THEN** Rust SHALL schedule a recurring check approximately once per hour while the application remains running

### Requirement: Native session search and export
The desktop runtime SHALL own persisted session search queries and filesystem export writes.

#### Scenario: Search persisted history
- **WHEN** the frontend searches historical sessions in desktop mode
- **THEN** Rust SHALL query SQLite for session metadata and message content and return bounded results

#### Scenario: Write export file
- **WHEN** the frontend requests desktop session export with a selected destination directory
- **THEN** Rust SHALL serialize the requested session and write the JSON or Markdown file to that directory

### Requirement: Native file reference validation
The desktop runtime SHALL validate chat file references against the owning session root before including file content in an Agent prompt.

#### Scenario: Validate referenced file
- **WHEN** a message includes file references
- **THEN** Rust SHALL confirm each file resolves inside the session root and satisfies size and text-content limits before reading it

#### Scenario: Log unsafe reference rejection
- **WHEN** a file reference is rejected for safety or availability reasons
- **THEN** Rust SHALL return a concise user-displayable error and write redacted diagnostics through unified logging

### Requirement: Native Prompt Hook persistence
The native runtime SHALL persist Prompt Hook overrides, user-created hooks, CLI bindings, and recent trace summaries in SQLite through additive migrations.

#### Scenario: Migrate Prompt Hook storage
- **WHEN** the native runtime opens an empty or older VaneHub database
- **THEN** it SHALL add Prompt Hook storage without deleting or rewriting existing agents, settings, sessions, messages, CLI statuses, Skills, SDK data, MCP data, IM data, or usage records

#### Scenario: Persist hook mutation atomically
- **WHEN** a Prompt Hook mutation updates enabled state, user hook content, metadata, or CLI bindings
- **THEN** the native runtime SHALL validate the complete mutation and commit it atomically

#### Scenario: Reject invalid hook mutation
- **WHEN** a Prompt Hook mutation contains invalid manifest data, unsupported category, unsupported stable agent id, unsafe content, or an immutable built-in edit
- **THEN** the native runtime SHALL reject the complete mutation
- **AND** it SHALL retain the previously committed state

### Requirement: Native Prompt Hook pipeline
The native runtime SHALL provide a provider-agnostic Prompt Hook pipeline before CLI provider invocation.

#### Scenario: Assemble effective prompt
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the native runtime SHALL evaluate enabled hooks bound to that stable agent id in deterministic stage and order
- **AND** it SHALL produce one effective prompt for the provider invocation builder

#### Scenario: Preserve provider-specific launch ownership
- **WHEN** Prompt Hook assembly completes
- **THEN** provider-specific command construction, stdin or argument prompt delivery, session resume tokens, and CLI parameter mapping SHALL remain owned by the provider invocation builder

#### Scenario: Avoid script execution
- **WHEN** the Prompt Hook pipeline renders built-in or user-created hooks
- **THEN** it SHALL treat hook templates as prompt text
- **AND** it SHALL NOT execute hook-provided shell commands, scripts, or arbitrary code

### Requirement: Native Prompt Hook commands remain bounded
Native Prompt Hook management and preview commands SHALL remain bounded request/response operations.

#### Scenario: Return Prompt Hook list directly
- **WHEN** the frontend lists Prompt Hooks or recent trace summaries
- **THEN** the native command MAY return the result directly after bounded catalog and SQLite reads
- **AND** it SHALL NOT spawn external commands, access networks, or launch provider CLIs

#### Scenario: Preview without provider launch
- **WHEN** the frontend requests Prompt Hook or effective prompt preview
- **THEN** the native runtime SHALL render the preview without launching a provider CLI process

### Requirement: Native settings commands for local data and startup
The native runtime SHALL expose settings-adapter commands for opening the SQLite database directory and managing launch-on-startup registration.

#### Scenario: Open database directory from native command
- **WHEN** the Tauri settings adapter requests opening the database directory
- **THEN** the native runtime SHALL resolve the active SQLite database path from the registry store and open its containing directory
- **AND** it SHALL NOT expose direct SQLite access to React components

#### Scenario: Return database location information
- **WHEN** the Tauri settings adapter requests settings or data-management metadata
- **THEN** the native runtime SHALL provide user-displayable database location information without requiring the frontend to infer app data paths

#### Scenario: Manage startup registration from native command
- **WHEN** the Tauri settings adapter saves launch-on-startup
- **THEN** the native runtime SHALL synchronize the official Tauri autostart registration and return success or a sanitized user-displayable failure

#### Scenario: Preserve command boundary errors
- **WHEN** database-directory opening or startup registration fails across the Tauri command boundary
- **THEN** the command SHALL convert the error to `Result<T, String>` or the project's command-safe error shape
