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
The native runtime SHALL execute external commands only through backend-owned command construction or validated user configuration without shell string interpolation. Command construction SHALL NOT permit the operating system to attach interactive console UI to a child process, so a launch failure stays on the application's own error path.

#### Scenario: Backend-owned command
- **WHEN** the native runtime launches a known Agent or SDK command from backend-owned metadata
- **THEN** it SHALL construct the process invocation with explicit executable and argument values

#### Scenario: User-configured command
- **WHEN** the native runtime runs a user-configured MCP command
- **THEN** it SHALL validate the command configuration, avoid shell string interpolation, and record an audit log entry for the execution attempt

#### Scenario: Console-subsystem child is launched
- **WHEN** the native runtime constructs a command for a console-subsystem executable
- **THEN** the child SHALL NOT be given a console window

#### Scenario: A launched command fails to start
- **WHEN** launching an external command fails
- **THEN** the failure SHALL be returned to the calling native code as a handled error
- **AND** no operating-system component SHALL present a dialog the application cannot dismiss or record

#### Scenario: Capability detection runs at startup
- **WHEN** startup detection probes for CLI availability
- **THEN** those probes SHALL NOT make windows appear on the user's desktop

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
The desktop runtime SHALL run one-shot session recovery reconciliation in Rust after database, unified logging, session repositories, and runtime evidence adapters are initialized, and SHALL run recurring archival and retention maintenance separately.

#### Scenario: Start maintenance jobs
- **WHEN** the desktop runtime initializes successfully
- **THEN** it SHALL attach the runtime and evidence adapters before reconciling interrupted sessions
- **AND** it SHALL NOT classify sessions as orphaned merely because the runtime adapter has not yet been attached

#### Scenario: Reconcile before dependent runtimes
- **WHEN** startup contains ordinary sessions owned or referenced by Plan or Loop execution
- **THEN** ordinary session evidence reconciliation SHALL complete before Plan and Loop project their recovery outcomes

#### Scenario: Start recurring maintenance after recovery
- **WHEN** startup recovery and dependent Plan/Loop projection have completed or safely deferred retryable storage work
- **THEN** Rust SHALL start automatic archival and retention schedules without combining those mutations with recovery decisions

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
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli`
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

### Requirement: Native bounded-context ownership
The Rust native runtime SHALL organize domain-bearing behavior into documented bounded contexts, and each business rule, use case, persistence model, and integration SHALL have one explicit owning context.

#### Scenario: Add native domain behavior
- **WHEN** a new native capability or business rule is implemented
- **THEN** it SHALL be assigned to a named bounded context with vocabulary and ownership consistent with that context
- **AND** it SHALL NOT be added to a root module or generic utility module solely for cross-feature convenience

#### Scenario: Use behavior owned by another context
- **WHEN** one bounded context needs behavior or data owned by another bounded context
- **THEN** it SHALL use the owning context's published application API, immutable contract, or explicit event
- **AND** it SHALL NOT import the other context's repository, infrastructure adapter, private aggregate, or Tauri command DTO

### Requirement: Inward native dependency direction
Native context dependencies MUST point inward from interface and infrastructure adapters to application use cases and domain code, while domain and application layers SHALL remain independent of concrete runtime frameworks.

#### Scenario: Compile domain code
- **WHEN** a bounded context's domain layer is compiled or tested
- **THEN** it SHALL NOT depend on Tauri, SQLite, filesystem, network, external-process, OS credential, task-registry, or logging implementations
- **AND** it SHALL NOT depend on command, infrastructure, bootstrap, or another context's private modules

#### Scenario: Compile application code
- **WHEN** a bounded context's application layer is compiled or tested
- **THEN** it SHALL depend only on its domain model, explicit application ports, and deliberately published cross-context contracts
- **AND** it SHALL NOT depend on Tauri state, Tauri commands, Rusqlite connections, or concrete filesystem, network, process, credential, logging, or task adapters

#### Scenario: Implement an external integration
- **WHEN** SQLite, filesystem, network, process, credential, unified-log, task, or desktop-runtime behavior is required by a use case
- **THEN** an outer infrastructure or interface adapter SHALL implement a narrow port owned by the consuming application layer

### Requirement: Explicit domain and boundary models
The native runtime SHALL model business invariants in domain types and SHALL keep domain models distinct from Tauri transport DTOs and SQLite row representations whenever transport or persistence concerns differ from domain semantics.

#### Scenario: Accept a Tauri command payload
- **WHEN** a Tauri command receives serialized input
- **THEN** the interface adapter SHALL validate and map that payload into application or domain input before executing business behavior
- **AND** deserialization alone SHALL NOT bypass domain invariants

#### Scenario: Load persisted domain state
- **WHEN** an infrastructure repository reads SQLite rows
- **THEN** it SHALL map those rows into valid domain types through explicit conversion
- **AND** SQLite column details SHALL NOT become domain-layer dependencies

#### Scenario: Reject an invalid state transition
- **WHEN** a requested mutation violates a domain invariant
- **THEN** the domain or application layer SHALL return a typed error without performing the invalid persistence or external side effect

### Requirement: Use-case and port boundaries
Native business workflows SHALL be exposed as application use cases with context-specific ports, and external entry points SHALL delegate to those use cases without implementing business or persistence logic.

#### Scenario: Handle a Tauri command
- **WHEN** a declared Tauri command is invoked
- **THEN** its handler SHALL map transport input, invoke one or more explicit application use cases, map command-safe output or errors, and perform only interface-owned event emission
- **AND** it SHALL NOT execute SQL, construct external processes, or decide domain policy directly

#### Scenario: Define a persistence port
- **WHEN** an application use case needs persisted state
- **THEN** it SHALL depend on a behavior-oriented, context-owned repository or transaction port
- **AND** that port SHALL NOT expose raw SQL rows, a Rusqlite connection, or generic CRUD as the cross-layer contract

#### Scenario: Execute an atomic mutation
- **WHEN** a use case requires multiple owned persistence changes to succeed or fail together
- **THEN** its application and infrastructure boundaries SHALL preserve one explicit atomic transaction boundary

### Requirement: Native composition root
The native runtime SHALL construct concrete repositories, gateways, use cases, and interface state in a dedicated composition root, while `lib.rs` SHALL remain limited to module exposure and delegation to native bootstrap.

#### Scenario: Start the Tauri desktop runtime
- **WHEN** native setup completes storage and migration initialization
- **THEN** the composition root SHALL construct concrete adapters and inject them into assembled application services registered with Tauri state
- **AND** domain and application code SHALL NOT resolve dependencies from Tauri state or a global service locator

#### Scenario: Register Tauri commands
- **WHEN** the native application builds its invoke handler
- **THEN** command registration SHALL be centralized and auditable by bounded-context command group
- **AND** each command implementation SHALL remain in the native interface layer

### Requirement: DDD refactor compatibility
The native DDD migration SHALL preserve existing external contracts, persisted data, unified logging, and observable operation behavior unless a separate approved specification explicitly changes them.

#### Scenario: Migrate a native command
- **WHEN** an existing Tauri command is moved behind a new context use case
- **THEN** its command name, request and response serialization, command-safe error behavior, and frontend service contract SHALL remain compatible

#### Scenario: Open an existing database after migration
- **WHEN** a user starts the refactored runtime with an existing supported SQLite database
- **THEN** all previously valid data SHALL remain readable through versioned migrations
- **AND** module or context renaming SHALL NOT cause destructive schema changes

#### Scenario: Migrate a logged long-running operation
- **WHEN** an operation or task implementation moves to the new architecture
- **THEN** its stable operation id, lifecycle state, terminal result or error, available page output, unified-log association, and redaction behavior SHALL remain available

#### Scenario: Use the browser runtime during native refactor
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL remain usable without importing or invoking the refactored Rust runtime

### Requirement: Executable native architecture verification
The project SHALL enforce native dependency boundaries through Rust visibility and an automated architecture check included in the normal Rust test workflow.

#### Scenario: Introduce a forbidden dependency
- **WHEN** domain or application source imports a forbidden outer-layer framework, adapter, or private cross-context module
- **THEN** the automated architecture check SHALL fail with the source location and violated dependency rule

#### Scenario: Verify a bounded context
- **WHEN** native tests run for a migrated context
- **THEN** domain tests SHALL run without live infrastructure, application tests SHALL use deterministic port doubles, and infrastructure tests SHALL cover its SQLite or external-adapter mappings where applicable

#### Scenario: Verify interface compatibility
- **WHEN** a Tauri handler is migrated
- **THEN** contract tests SHALL verify its serialized DTO shape and command-safe error mapping before the legacy path is removed

### Requirement: Project-level Rust DDD standards
The project standards SHALL document the native bounded-context map, target module layout, layer responsibilities, allowed dependency direction, model-mapping rules, port and transaction conventions, error and logging boundaries, testing expectations, and exception process.

#### Scenario: Implement new Rust native work
- **WHEN** an implementation task adds or materially changes native domain behavior
- **THEN** it SHALL follow the DDD rules in `openspec/project.md` and place the behavior in its owning bounded context

#### Scenario: Request a temporary architecture exception
- **WHEN** a migration cannot immediately comply with one documented dependency rule
- **THEN** the exception SHALL be narrow, justified, recorded with an owning migration task, and removed before that context's migration is considered complete

#### Scenario: Review a migrated context
- **WHEN** a bounded-context migration is submitted for completion
- **THEN** reviewers SHALL be able to identify its domain model, application use cases and ports, outer adapters, public context API, transaction ownership, and verification coverage from the documented structure

### Requirement: Native Agent terminal ownership
The Rust native runtime SHALL own Agent Terminal launch, attach, input, resize, stop, idle cleanup, shutdown cleanup, and diagnostics behind bounded context application use cases and Tauri command adapters.

#### Scenario: Handle terminal command
- **WHEN** a frontend adapter requests Agent Terminal start, attach, input, resize, stop, or event subscription
- **THEN** the Tauri command layer SHALL map the transport request into an application use case
- **AND** it SHALL NOT construct shell commands, execute SQL, or decide Agent runtime policy in the command handler

#### Scenario: Keep React isolated
- **WHEN** React UI code renders or controls the Agent Terminal
- **THEN** it SHALL call the frontend service interface
- **AND** Tauri `invoke()` usage SHALL remain inside Tauri-specific frontend adapters

### Requirement: Native shell wrapper safety
The native runtime SHALL construct Agent Terminal shell wrappers without frontend-supplied shell strings and SHALL record only redacted command diagnostics.

#### Scenario: Generate wrapper from validated tokens
- **WHEN** an Agent Terminal process is launched
- **THEN** the native runtime SHALL resolve the CLI executable and validated argument tokens before writing or invoking a shell wrapper
- **AND** it SHALL NOT accept an arbitrary shell command string from React components

#### Scenario: Redact launch diagnostics
- **WHEN** the native runtime records Agent Terminal launch diagnostics
- **THEN** it SHALL redact prompts, credentials, tokens, secret-like values, and sensitive runtime context before persistence
- **AND** it SHALL write diagnostics through the unified logging service

### Requirement: Agent terminal registry cleanup
The native runtime SHALL maintain a bounded registry of live Agent Terminal processes and clean it up deterministically.

#### Scenario: One live terminal per session
- **WHEN** a start request targets a session with an existing live Agent Terminal process
- **THEN** the native runtime SHALL attach to the existing process rather than spawning a second Agent CLI process for that session

#### Scenario: Serialize same-session starts
- **WHEN** two start requests for the same session arrive before the first process launch has finished registering
- **THEN** the native runtime SHALL serialize the open-or-attach path through the terminal registry
- **AND** it SHALL create no more than one live Agent Terminal process for that session

#### Scenario: Cleanup idle terminal
- **WHEN** a live Agent Terminal process exceeds the configured two-hour inactivity threshold
- **THEN** the native runtime SHALL stop the process and remove its live registry entry
- **AND** it SHALL preserve persisted session metadata needed for later resume

#### Scenario: Cleanup on shutdown
- **WHEN** the desktop runtime begins application shutdown
- **THEN** it SHALL stop all live Agent Terminal processes before shutdown completes when possible
- **AND** cleanup failures SHALL be logged through unified logging with redaction

### Requirement: Native Prompt Hook version persistence
The native runtime SHALL persist Prompt Hook drafts and immutable published versions through additive SQLite migrations owned by `tooling::prompt_hooks`.

#### Scenario: Migrate existing user Hooks
- **WHEN** an existing database is opened after the versioning migration
- **THEN** each existing user Hook SHALL retain its identity, enabled state, bindings, metadata, template, and version as the selected published snapshot
- **AND** the migration SHALL NOT delete or rewrite unrelated application data

#### Scenario: Publish atomically
- **WHEN** a valid draft is published
- **THEN** appending the immutable version, selecting it, and consuming the matching draft revision SHALL succeed or fail in one transaction

#### Scenario: Query bounded history
- **WHEN** the frontend requests one Hook's version history and evaluation summaries
- **THEN** a bounded native command SHALL query through the Prompt Hook application and repository ports
- **AND** the command handler SHALL NOT contain SQL or domain policy

### Requirement: Native Prompt Hook evaluation persistence
The native runtime SHALL persist idempotent safe execution observations and compute bounded version aggregates without loading Prompt or response bodies.

#### Scenario: Record one terminal observation
- **WHEN** `agent_runtime` reports a terminal invocation outcome through the Prompt Hook published API
- **THEN** the Prompt Hook application service SHALL persist at most one observation for each invocation id, Hook id, and version
- **AND** the write SHALL occur outside the Tauri main thread completion path

#### Scenario: Keep evaluation records safe
- **WHEN** evaluation data is stored, queried, or included in unified diagnostics
- **THEN** it SHALL contain only stable ids, version, outcome, elapsed milliseconds, agent id, and timestamps
- **AND** it SHALL omit Prompt bodies, user or model content, raw errors, credentials, command arguments, and session content

### Requirement: Pooled SQLite connection reuse
The native runtime SHALL serve database operations from a bounded pool of reused, pre-configured SQLite connections instead of opening a new connection per operation, and SHALL initialize schema migration and registry seeding exactly once for the database.

#### Scenario: Reuse connections across operations
- **WHEN** the native runtime performs successive database operations
- **THEN** it SHALL check out an already-open connection from the pool rather than opening a new SQLite connection for each operation
- **AND** each pooled connection SHALL already have busy-timeout, foreign-key enforcement, and write-ahead logging configured

#### Scenario: One-time schema preparation
- **WHEN** the native runtime prepares the database during pool initialization
- **THEN** it SHALL apply versioned migrations and registry seeding exactly once
- **AND** concurrent first-use checkouts SHALL NOT apply migrations or seeding more than once

#### Scenario: Bounded connections under concurrent load
- **WHEN** more concurrent database operations are requested than the pool's maximum size
- **THEN** the runtime SHALL bound the number of live SQLite connections to the configured maximum
- **AND** excess requests SHALL wait for an available connection or fail with a structured timeout error rather than opening unbounded connections

### Requirement: Critical native path coverage
Native verification SHALL measure production Rust coverage and MUST maintain at least 80% line coverage for the designated Agent startup and terminal-control, MCP routing, and SQLite transaction path groups.

#### Scenario: Measure Agent runtime critical paths
- **WHEN** native coverage runs
- **THEN** the policy SHALL include Agent launch preparation, terminal open or attach, stop, startup failure, and cleanup behavior in the Agent critical-path group

#### Scenario: Measure MCP routing critical paths
- **WHEN** native coverage runs
- **THEN** the policy SHALL include supported routing, protocol forwarding, timeout, process failure, and bounded error behavior in the MCP critical-path group

#### Scenario: Measure SQLite transaction critical paths
- **WHEN** native coverage runs
- **THEN** the policy SHALL include commit, rollback after a partial-write failure, pool contention, and migration compatibility behavior in the database critical-path group

#### Scenario: Coverage policy path is invalid
- **WHEN** a configured critical-path pattern matches no production Rust source
- **THEN** native coverage validation SHALL fail instead of silently treating the group as covered

### Requirement: Native Session and Agent Terminal lifecycle integration
The native test suite SHALL verify the supported Session and Agent Terminal lifecycle across published application/context boundaries with real temporary SQLite persistence and deterministic process doubles.

#### Scenario: Complete native lifecycle
- **WHEN** the integration test creates a Session, opens its Agent Terminal, observes running state, stops the terminal, and deletes the Session
- **THEN** persisted Session state, operation state, terminal registry state, and associated cleanup SHALL remain consistent after every transition

#### Scenario: Agent Terminal startup fails
- **WHEN** the deterministic process double fails while opening the Agent Terminal
- **THEN** the integration test SHALL verify a command-safe failure, failed lifecycle state, persisted redacted diagnostic association, and release of reserved runtime resources

#### Scenario: Lifecycle operation is repeated
- **WHEN** stop or cleanup is requested again after the Agent Terminal has already stopped
- **THEN** the integration test SHALL verify the documented idempotent result without recreating live runtime state

#### Scenario: Native integration remains deterministic
- **WHEN** the lifecycle integration suite runs on a supported CI host
- **THEN** it SHALL NOT require an installed provider CLI, external network service, user credential, persistent user database, or interactive Tauri window

### Requirement: Transaction rollback verification
Every newly covered multi-write SQLite consistency boundary SHALL include a deterministic failure-injection test proving that a failed later write does not leave earlier writes committed.

#### Scenario: Later write fails
- **WHEN** a deterministic SQLite trigger or repository double rejects a later write within one declared transaction
- **THEN** the test SHALL verify that all writes from that transaction are rolled back and pre-existing data remains unchanged

#### Scenario: Transaction succeeds
- **WHEN** all writes in the declared consistency boundary succeed
- **THEN** the test SHALL verify that all related state becomes visible together after commit

### Requirement: Bounded native IM work ownership
The native communications context SHALL own explicit bounds for admitted pending messages, active IM Agent generations, completion receivers, and retained per-chat lane state.

#### Scenario: IM traffic exceeds native capacity
- **WHEN** inbound traffic across distinct external chats exceeds the configured IM admission bound
- **THEN** the communications runtime SHALL reject excess work through the connector's bounded busy behavior without creating unbounded tasks, blocking workers, or lane entries

#### Scenario: IM work drains
- **WHEN** admitted work reaches a terminal state or is rejected, cancelled, or timed out
- **THEN** its global capacity reservation, completion registration, and idle lane state SHALL be released exactly once

### Requirement: Failure-isolated connector lifecycle coordination
The native communications context SHALL coordinate lifecycle mutations per connector so one connector's slow or failed operation does not corrupt or block unrelated connectors.

#### Scenario: Replace connector runtime
- **WHEN** an enabled connector receives a validated configuration update
- **THEN** the runtime manager SHALL stop and replace the registered adapter through one coordinated operation and SHALL NOT orphan the previous worker

#### Scenario: One connector startup fails
- **WHEN** startup or shutdown of one enabled connector fails
- **THEN** the runtime SHALL continue attempting the requested lifecycle operation for other connectors and SHALL return or log connector-scoped safe outcomes

### Requirement: Native custom instructions CLI injection precedes Prompt Hook assembly in the final effective prompt

The native runtime SHALL combine host-level custom instructions with the Prompt Hook pipeline's assembled output into one final effective prompt, before that text reaches the provider invocation builder. This requirement governs only where custom instructions are combined relative to the Prompt Hook pipeline; the "Native Prompt Hook pipeline" requirement's own hook evaluation, binding, and template rendering are unaffected.

#### Scenario: Combine custom instructions ahead of the Prompt Hook output
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli` with custom instructions enabled and non-empty
- **THEN** the native runtime SHALL place the custom-instructions section before the Prompt Hook pipeline's assembled content in the final effective prompt handed to the provider invocation builder

#### Scenario: No custom instructions leaves Prompt Hook assembly unchanged
- **WHEN** custom instructions are disabled or empty
- **THEN** the final effective prompt SHALL be exactly the Prompt Hook pipeline's own assembled output, unchanged from behavior before this requirement existed

#### Scenario: Custom instructions resolution failure does not block CLI invocation
- **WHEN** resolving custom instructions fails during a CLI chat invocation
- **THEN** the native runtime SHALL log the failure and proceed with the Prompt Hook pipeline's assembled output alone
- **AND** it SHALL NOT fail or delay the CLI invocation

### Requirement: Native memory injection follows custom instructions and precedes Prompt Hook assembly in the final CLI effective prompt

The native runtime SHALL combine the shared host-level memory pool with the Prompt Hook pipeline's assembled output into the final effective prompt for CLI-wrapped agents, placed after any custom-instructions section and before the Prompt Hook pipeline's own assembled content, before that text reaches the provider invocation builder. This requirement governs only where the memory section sits relative to custom instructions and the Prompt Hook pipeline; the "Native Prompt Hook pipeline" requirement's own hook evaluation, binding, and template rendering are unaffected, and the "Native custom instructions CLI injection precedes Prompt Hook assembly in the final effective prompt" requirement's own ordering guarantee is unaffected.

#### Scenario: Combine memory content between custom instructions and the Prompt Hook output
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli` with the memory enablement toggle on and at least one memory in the shared pool
- **THEN** the native runtime SHALL place the memory section after the custom-instructions section (if present) and before the Prompt Hook pipeline's assembled content in the final effective prompt handed to the provider invocation builder

#### Scenario: No memory content leaves the rest of the effective prompt unchanged
- **WHEN** the memory enablement toggle is off, or the shared memory pool is empty
- **THEN** the final effective prompt SHALL be exactly what it would have been without this requirement, unchanged from behavior before this requirement existed

#### Scenario: Memory resolution failure does not block CLI invocation
- **WHEN** resolving the shared memory pool fails during a CLI chat invocation
- **THEN** the native runtime SHALL log the failure and proceed with the rest of the effective prompt (custom instructions and Prompt Hook output) unaffected
- **AND** it SHALL NOT fail or delay the CLI invocation

### Requirement: Contained external command termination

The native runtime SHALL spawn every externally-executed command on the bounded execution path into a platform process-containment primitive so the runtime can reach processes that command spawns. When the runtime terminates such a command because it exceeded its timeout or was cancelled, it SHALL terminate the entire contained process tree rather than only the process it launched directly.

The runtime SHALL continue to decide that a command has *completed* from the exit of the process it launched directly, and SHALL NOT wait for that process's descendants before returning a result. A command that exits successfully SHALL NOT have its surviving descendants terminated, so callers that deliberately launch a background process keep today's behavior.

#### Scenario: Timed-out command leaves a descendant running

- **WHEN** an external command on the bounded execution path exceeds its timeout and the process it launched has itself spawned another process
- **THEN** the native runtime SHALL terminate both the launched process and its descendants
- **AND** it SHALL report the timeout failure to the caller as it does today

#### Scenario: Cancelled command leaves a descendant running

- **WHEN** an external command on the bounded execution path is cancelled and the process it launched has itself spawned another process
- **THEN** the native runtime SHALL terminate both the launched process and its descendants
- **AND** it SHALL report the cancellation to the caller as it does today

#### Scenario: Successful command leaves a background process

- **WHEN** an external command on the bounded execution path exits successfully while a process it spawned is still running
- **THEN** the native runtime SHALL report the command as completed without waiting for that surviving process
- **AND** it SHALL NOT terminate that surviving process

#### Scenario: Containment is unavailable for a command

- **WHEN** the native runtime cannot establish process containment for a command it is about to launch
- **THEN** it SHALL NOT leave a started process unsupervised, and SHALL surface the failure as a launch failure rather than silently running the command without containment

### Requirement: Antigravity CLI built-in agent registration
The native runtime SHALL register `antigravity-cli` as a built-in Agent whose provider is Google, whose launch kind is `cli`, whose launch command and executable name are both `agy`, and whose supported interaction modes are `cli`. Registration SHALL be idempotent for databases that already contain the row, and SHALL NOT require the `agy` executable to be present on the host.

#### Scenario: Built-in agent present after upgrade
- **WHEN** the native runtime starts against a database created before this change
- **THEN** the agent registry SHALL contain an `antigravity-cli` entry with agent origin `builtin`
- **AND** re-running the same startup against a database that already has the row SHALL leave it unchanged

#### Scenario: Availability reported without the executable installed
- **WHEN** an availability check runs for `antigravity-cli` on a host where `agy` is not on PATH and no known install directory contains it
- **THEN** the runtime SHALL report the agent as unavailable with the reason naming the missing `agy` command
- **AND** the check SHALL NOT start an interactive session or a CLI process

### Requirement: Antigravity CLI managed chat invocation contract
The native runtime SHALL build managed (non-interactive) chat invocations for `antigravity-cli` as `agy` invoked with the agent's mapped CLI parameters, `--output-format stream-json`, the prompt delivered through the `-p` argument, and — when a provider runtime session id is known — `--conversation <id>` to resume that conversation. The runtime SHALL NOT expose `-p`, `--output-format`, or `--conversation` as user-selectable parameters.

#### Scenario: Build a fresh invocation
- **WHEN** a CLI chat invocation starts for `antigravity-cli` with no persisted runtime session id
- **THEN** the built argument list SHALL contain `--output-format stream-json` and deliver the effective prompt as the `-p` argument value
- **AND** it SHALL NOT contain `--conversation`

#### Scenario: Resume a known conversation
- **WHEN** a CLI chat invocation starts for `antigravity-cli` for a session with a persisted runtime session id
- **THEN** the built argument list SHALL contain `--conversation` followed by that id

#### Scenario: Managed arguments cannot be overridden by user selections
- **WHEN** a saved CLI parameter profile for `antigravity-cli` would produce `-p`, `--output-format`, or `--conversation`
- **THEN** the invocation builder SHALL reject or drop that selection rather than emit a duplicate or conflicting argument

### Requirement: Antigravity CLI streaming output normalization
The native runtime SHALL parse `antigravity-cli` stdout as newline-delimited JSON carrying `init`, `step_update`, and `result` events, and SHALL normalize them into the runtime's existing chat event vocabulary. The runtime SHALL treat unrecognized event kinds and unrecognized fields within a recognized event as ignorable rather than as parse failures.

#### Scenario: Capture the runtime session id
- **WHEN** an `init` event carries a `conversation_id`
- **THEN** the runtime SHALL persist that value as the session's provider runtime session id

#### Scenario: Terminal status determines the lifecycle outcome
- **WHEN** a `result` event reports status `SUCCESS`
- **THEN** the invocation SHALL complete successfully, carrying the reported usage
- **AND** **WHEN** it reports `ERROR`, `INVALID`, `CANCELED`, or `INTERRUPTED`, the invocation SHALL fail non-retryably with the event's own reported error preserved as the diagnostic

#### Scenario: A self-reported cancel is not silently treated as success
- **WHEN** a `result` event reports status `CANCELED` or `INTERRUPTED`
- **THEN** the invocation SHALL NOT report a completed turn
- **AND** the failure SHALL be classified non-retryable, because re-running cannot resolve a cancellation the provider decided on

#### Scenario: Non-terminal status on a terminal event is a protocol violation
- **WHEN** a `result` event reports status `WAITING` or `RUNNING`
- **THEN** the runtime SHALL fail the invocation with a protocol error rather than treat it as success or silently discard it

#### Scenario: Unknown event kinds do not break a run
- **WHEN** stdout contains a JSON line whose event kind the runtime does not recognize
- **THEN** the runtime SHALL ignore that line and continue processing subsequent events

#### Scenario: Incremental step events are consumed without inventing a payload shape
- **WHEN** stdout contains a `step_update` event
- **THEN** the runtime SHALL consume it without emitting incremental output, until its payload has been captured from a live authenticated run
- **AND** the completed turn SHALL still deliver the full reply, which the `result` event carries in its `response` field

### Requirement: Child process reaping must not hold the child lock across the blocking wait

A native monitor or stop path that reaps a managed child process held behind an `Arc<Mutex<…>>` SHALL NOT call the blocking `wait()` while holding that lock. It SHALL poll `try_wait()` with short lock holds so a concurrent cancellation path can acquire the lock to `kill()` the child. A drain that joins its worker thread after the child has been shut down SHALL bound that join with a deadline and abandon the worker on timeout rather than blocking indefinitely.

#### Scenario: CLI closes stdout but stays alive and the user cancels

- **WHEN** a managed CLI monitor reads stdout to EOF while the child process is still alive, and a stop request arrives to cancel it
- **THEN** the stop request SHALL acquire the child lock, kill the child, and complete rather than deadlock against the monitor's blocking wait
- **AND** the monitor SHALL eventually reap the child once `try_wait()` reports it exited

#### Scenario: Grandchild holds the stderr pipe after the child is shut down

- **WHEN** a managed MCP stdio relay shuts its child down and a grandchild that inherited the stderr pipe keeps the drain reader pending
- **THEN** the drain finish SHALL time out and abandon the worker rather than wedge the relay shutdown
- **AND** the abandoned worker SHALL terminate on its own once the pipe's last writer closes

#### Scenario: A process is monitored twice

- **WHEN** `monitor_generation` is called twice for the same process id
- **THEN** the second call SHALL be rejected rather than spawning a duplicate generation thread, mirroring the guard the CLI adapter already has

### Requirement: Migration application is transactional with startup density verification

The native runtime SHALL apply each SQLite migration inside a single transaction that records the version row on the same commit as the schema change, so a mid-migration failure rolls back both. After applying migrations, the runtime SHALL verify the recorded `schema_migrations` history is dense and within the expected version range, surfacing a diverged history as an explicit startup error.

#### Scenario: A migration fails partway through

- **WHEN** a migration's DDL or DML fails before completion
- **THEN** the transaction SHALL roll back so no partial schema change is committed without its version row

#### Scenario: A migration version was silently skipped

- **WHEN** the recorded `schema_migrations` history is not dense or exceeds the highest version the binary expects
- **THEN** startup SHALL fail with an explicit, diagnosable error rather than a later opaque "no such table" crash

### Requirement: Command errors redact forwarded lower-layer messages at the boundary

A `From<…> for CommandError` implementation that forwards a lower-layer message verbatim (e.g. an internal/infrastructure/repository variant whose payload may carry a filesystem path or provider diagnostic) SHALL redact that payload at the conversion boundary. Structured category-level error codes that are safe and matched by the frontend SHALL pass through unchanged. Command families that previously returned `Result<T, String>` via `to_string()` SHALL route through `CommandError` so the same redaction applies.

#### Scenario: A path-bearing CLI config error reaches the frontend

- **WHEN** a `cli_config` parse or filesystem error carries an absolute path and is returned to the frontend
- **THEN** the command SHALL surface a fixed category-level message and SHALL NOT forward the path

#### Scenario: A structured error code is returned

- **WHEN** a command error is a safe category-level code (e.g. `connector-credentials-required`)
- **THEN** the code SHALL be returned unchanged, not mangled by heuristic redaction

### Requirement: Recovery consistency boundaries are failure-injected
The native test suite SHALL verify recovery-critical multi-write transactions and conditional publications against deterministic failures and database reopen.

#### Scenario: Fail a later recovery write
- **WHEN** a failure is injected after an earlier write within a recovery-critical transaction
- **THEN** reopening the file-backed test database SHALL show either the complete transaction or none of it, without a partially published recovery decision

#### Scenario: Reopen after a simulated crash point
- **WHEN** execution is interrupted after a durable generation or recovery transition
- **THEN** a newly constructed runtime SHALL reconcile the reopened database idempotently without relying on the previous process's memory

### Requirement: Mechanically enforced native dependency direction
Native architecture fitness SHALL parse production Rust sources and reject domain or application dependencies on forbidden outer technologies or layers, and reject cross-context access to private modules.

#### Scenario: Domain imports infrastructure
- **WHEN** a domain module imports its own infrastructure layer, a concrete platform adapter, Tauri, SQLite, filesystem, process, or network APIs
- **THEN** native architecture fitness SHALL fail with the native dependency rule id, file, line, and dependency path

#### Scenario: Application imports an outer layer
- **WHEN** an application module imports concrete infrastructure, command state, Tauri, or a concrete SQLite connection
- **THEN** native architecture fitness SHALL fail with the native dependency rule id, file, line, and dependency path

#### Scenario: Context imports another context privately
- **WHEN** one bounded context imports another context's repository, infrastructure, private aggregate, or other non-API module
- **THEN** native architecture fitness SHALL fail and direct the caller to the owning context's published API, contract, or event

### Requirement: Mechanically enforced native adapter thinness
Native architecture fitness SHALL reject Tauri command handlers that execute SQL, construct external processes, or contain business-policy control flow, and SHALL reject concrete runtime I/O assembly outside bootstrap.

#### Scenario: Command performs concrete I/O
- **WHEN** a Tauri command contains SQL, opens a concrete database connection, or constructs or executes an external process
- **THEN** native architecture fitness SHALL fail with the command-thinness rule id and exact source location

#### Scenario: Concrete runtime is assembled outside bootstrap
- **WHEN** production native code outside bootstrap constructs a reviewed concrete runtime dependency that belongs to dependency assembly
- **THEN** native architecture fitness SHALL fail with the composition-root rule id and exact source location

### Requirement: Native architecture rules prove both outcomes
Native dependency, cross-context, command-thinness, and composition-root detectors SHALL each have syntax-valid positive and negative fixtures.

#### Scenario: Native fixture suite runs
- **WHEN** the native architecture test target executes
- **THEN** compliant fixtures SHALL be accepted and each prohibited dependency or I/O pattern SHALL be rejected with its rule id and location

### Requirement: Code review ownership across existing contexts
Native code review behavior SHALL remain in the existing modular-monolith contexts: `sessions` owns review records and feedback coordination, `workspaces` owns Git/path/fingerprint/revert policy, `operations` owns observable action lifecycle, and `permissions` owns destructive approval; cross-context calls SHALL use published APIs assembled in bootstrap.

#### Scenario: Review command executes
- **WHEN** a declared Tauri review command is invoked
- **THEN** the handler SHALL validate/map transport data and call assembled application services without SQL, Git process construction, or business policy in the command module

#### Scenario: Architecture fixtures inspect review code
- **WHEN** native architecture fitness tests scan the implementation
- **THEN** no review module SHALL import another context's private domain, repository, or infrastructure implementation

### Requirement: Complete native dependency enforcement
The native architecture test MUST inspect domain, application, infrastructure, and command source files and SHALL reject private cross-context infrastructure dependencies outside the composition root.

#### Scenario: Infrastructure imports another context repository
- **WHEN** a bounded context infrastructure module imports another context's concrete repository
- **THEN** the architecture test SHALL fail with the importing file, line, and dependency

#### Scenario: Command executes infrastructure behavior
- **WHEN** a command handler imports or invokes a context's private infrastructure implementation
- **THEN** the architecture test SHALL fail unless the command uses a deliberately published API contract
