# unified-log-management Specification

## Purpose
Defines the shared logging contract for diagnostic logs, operation logs, frontend client log events, log directory behavior, redaction, retention, archival, log levels, and future logging development requirements.

## Requirements
### Requirement: Unified log directory
The system SHALL manage one active log directory for native diagnostic logs and operation logs.

#### Scenario: Resolve default log directory
- **WHEN** no custom log directory has been saved
- **THEN** the desktop runtime SHALL resolve a default log directory under a VaneHub-owned app data path

#### Scenario: Change log directory
- **WHEN** a user saves a new log directory from common settings
- **THEN** the desktop runtime SHALL validate or create that directory and use it for newly written logs

#### Scenario: Preserve existing logs after path change
- **WHEN** the active log directory changes
- **THEN** the system SHALL NOT automatically move, copy, archive, or delete logs from the previous directory

### Requirement: Log directory opening
The system SHALL allow users to open the active log directory in the desktop runtime.

#### Scenario: Open configured log directory
- **WHEN** the user requests to open the log directory in the Tauri desktop runtime
- **THEN** the native runtime SHALL open the active log directory through a declared backend command

#### Scenario: Web runtime cannot open local directory
- **WHEN** the application runs through the Web/mock adapter
- **THEN** the system SHALL expose a mock log path and indicate that opening the local log directory is unavailable

### Requirement: Unified operation log persistence
The system SHALL persist SDK and CLI operation logs through the unified logging service.

#### Scenario: Persist SDK operation output
- **WHEN** an SDK install, update, rollback, or uninstall operation emits output
- **THEN** the native runtime SHALL write the operation output to the active log directory with SDK operation context

#### Scenario: Persist CLI operation output
- **WHEN** a CLI detection, install, upgrade, or downgrade operation emits output
- **THEN** the native runtime SHALL write the operation output to the active log directory with CLI operation context

#### Scenario: Preserve existing operation UI logs
- **WHEN** an SDK or CLI operation emits output
- **THEN** the existing settings page operation log display SHALL remain available through the frontend service boundary

### Requirement: Sensitive information redaction
The unified logging service SHALL redact sensitive information before writing log entries to disk.

#### Scenario: Redact built-in sensitive patterns
- **WHEN** a log entry contains password-like fields, token-like fields, API key fields, secret fields, bearer tokens, or common provider key formats
- **THEN** the persisted log entry SHALL replace the sensitive value with a redacted marker

#### Scenario: Redaction before persistence
- **WHEN** a native diagnostic or operation log entry is written
- **THEN** redaction SHALL occur before the entry is appended to a log file

### Requirement: Frontend client log events
Frontend events that require durable diagnostics SHALL be reported through the frontend service boundary and persisted by the native logging service in the desktop runtime.

#### Scenario: Report error boundary failure
- **WHEN** a React error boundary captures an unhandled UI rendering error
- **THEN** the frontend SHALL report a client log event through a frontend service interface rather than writing a local file directly

#### Scenario: Report critical user operation failure
- **WHEN** a critical user operation fails in the frontend and requires durable diagnostics
- **THEN** the frontend SHALL report the failure through a frontend service interface with client event context

#### Scenario: Persist desktop client event
- **WHEN** the Tauri desktop runtime receives a frontend client log event
- **THEN** the native runtime SHALL redact the event and write it to the active log directory through the unified logging service

#### Scenario: Web mock client event
- **WHEN** the Web/mock runtime receives a frontend client log event
- **THEN** the system SHALL NOT write a local log file and SHALL use no-op or mock behavior

### Requirement: Log retention and archival
The unified logging service SHALL automatically archive logs older than the fixed first-version retention window of 30 days.

#### Scenario: Archive expired logs
- **WHEN** the logging service detects log files older than 30 days
- **THEN** it SHALL move those logs into an archive location under the active log directory

#### Scenario: Keep recent logs active
- **WHEN** a log file is within the 30-day retention window
- **THEN** the system SHALL keep that log file available in the active log area

### Requirement: Log levels
The unified logging model SHALL support `error`, `warn`, `info`, and `debug` levels for persisted log entries.

#### Scenario: Persist log level
- **WHEN** a native diagnostic or operation log entry is written
- **THEN** the persisted entry SHALL include one of `error`, `warn`, `info`, or `debug`

#### Scenario: No user-facing level configuration
- **WHEN** the user opens Basic Settings
- **THEN** the system SHALL NOT provide a control for changing the active log level in the first version

### Requirement: Logging development contract
Future native features that emit diagnostics or operation logs SHALL use the unified logging service instead of writing feature-local log files or bypassing redaction.

#### Scenario: Add new operation flow
- **WHEN** a new native operation flow emits progress, stdout, stderr, diagnostics, or errors
- **THEN** its implementation SHALL write persistent logs through the unified logging service with an operation or feature context

#### Scenario: Avoid direct frontend filesystem logging
- **WHEN** a React component or frontend service needs log-related behavior
- **THEN** it SHALL use the frontend service boundary and SHALL NOT write local log files directly

### Requirement: Session runtime diagnostics use unified logs
Session-scoped Agent runtime diagnostics SHALL be persisted through the unified logging service in the desktop runtime.

#### Scenario: Persist runtime stdout and stderr
- **WHEN** a session Agent runtime emits stdout, stderr, command diagnostics, or exit status
- **THEN** the native runtime SHALL write those diagnostics through the unified logging service with session id and Agent id context

#### Scenario: Redact runtime diagnostics
- **WHEN** session runtime diagnostics contain sensitive information
- **THEN** the unified logging service SHALL redact sensitive values before writing them to disk

#### Scenario: Keep chat UI separate from diagnostics
- **WHEN** runtime diagnostics are persisted for a session
- **THEN** the chat UI SHALL continue to show user-facing messages through the message service contract
- **AND** raw diagnostic output SHALL NOT be exposed in React components by bypassing the service boundary

#### Scenario: Persist unsupported runtime diagnostics
- **WHEN** a selected Agent CLI is unavailable, not installed, unsupported, exits unsuccessfully, or is cancelled
- **THEN** the native runtime SHALL persist detailed diagnostics through unified logging
- **AND** the user-facing chat error SHALL remain concise

### Requirement: CLI refresh failure diagnostics
The system SHALL persist detailed CLI refresh and detection diagnostics through unified log management.

#### Scenario: Persist refresh operation failure
- **WHEN** a CLI detection refresh operation fails, times out, or partially completes with per-CLI errors
- **THEN** the native runtime SHALL write a redacted error or warning log entry to the active log directory with operation id, affected CLI id when available, command context, timeout context when applicable, and user-visible error summary

#### Scenario: Persist per-CLI detection failures
- **WHEN** executable resolution, CLI version probing, npm latest-version lookup, or npm version-list lookup fails for a managed CLI
- **THEN** the native runtime SHALL write a redacted diagnostic log entry with the CLI id, package name, executable name, attempted operation, stdout or stderr when available, and failure reason

### Requirement: CLI package operation failure diagnostics
The system SHALL persist detailed install, upgrade, and downgrade diagnostics for managed CLI npm package operations through unified log management.

#### Scenario: Persist npm install failure details
- **WHEN** a CLI package install, upgrade, or downgrade operation fails for Claude Code, OpenCode, Codex CLI, or Gemini CLI
- **THEN** the native runtime SHALL write a redacted error log entry with operation id, CLI id, package name, target version, npm executable, explicit npm arguments, stdout, stderr, exit status or timeout reason, and sanitized environment context

#### Scenario: Keep concise user-facing package error
- **WHEN** detailed CLI package failure diagnostics are persisted
- **THEN** the frontend SHALL receive a concise user-displayable operation error
- **AND** detailed diagnostics SHALL remain available in the active log directory rather than being exposed by bypassing the service boundary

### Requirement: CLI chat runtime diagnostics use unified logging
Desktop CLI chat runtime diagnostics SHALL be persisted through the unified logging service with redaction before disk writes.

#### Scenario: Persist CLI chat stdout and stderr diagnostics
- **WHEN** a provider CLI chat invocation emits stdout, stderr, lifecycle, cancellation, timeout, or failure diagnostics
- **THEN** the desktop runtime SHALL write diagnostic log entries through the unified logging service
- **AND** the entries SHALL include session id, agent id, and runtime context where available

#### Scenario: Redact prompt and secrets
- **WHEN** CLI chat runtime diagnostics contain prompt text, token-like values, API keys, bearer tokens, password-like fields, or secret-like fields
- **THEN** the persisted log entry SHALL redact sensitive values before writing to disk
- **AND** command audit logs SHALL avoid storing raw prompt text

#### Scenario: Keep chat output user-facing
- **WHEN** detailed provider diagnostics are written to unified logs
- **THEN** the chat UI SHALL show concise user-facing errors instead of raw stderr dumps
- **AND** successfully streamed assistant text SHALL remain visible in the message list
