## ADDED Requirements

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

