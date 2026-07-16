## ADDED Requirements

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
