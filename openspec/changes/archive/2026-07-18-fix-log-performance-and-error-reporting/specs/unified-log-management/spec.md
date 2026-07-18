## ADDED Requirements

### Requirement: Structured diagnostic redaction
The unified logging service SHALL redact sensitive values from native diagnostic messages and context before writing to disk or emitting an emergency diagnostic sink.

#### Scenario: Redact JSON and whitespace-separated values
- **WHEN** a diagnostic contains a sensitive key with `=`, `:`, JSON quoting, or whitespace between the key and value
- **THEN** the persisted entry SHALL replace the sensitive value with a redacted marker
- **AND** the original sensitive value SHALL NOT appear in the message, context, or native stderr output

#### Scenario: Redact bearer and provider credentials
- **WHEN** a diagnostic contains a bearer credential or a supported provider-token prefix
- **THEN** the persisted entry SHALL replace the credential with a redacted marker before it reaches any diagnostic sink

### Requirement: Active log rotation and efficient retention
The unified logging service SHALL rotate active logs and perform retention maintenance without scanning the log directory for every log entry.

#### Scenario: Rotate active log before retention age
- **WHEN** the active log crosses the configured rotation boundary during a write
- **THEN** the service SHALL rename it to a timestamped active-directory log file
- **AND** subsequent entries SHALL be appended to a fresh active log file

#### Scenario: Archive rotated expired logs
- **WHEN** scheduled retention maintenance finds a rotated log older than 30 days
- **THEN** the service SHALL move that rotated log into the active log directory archive location

#### Scenario: Avoid per-entry directory scans
- **WHEN** multiple log entries are appended within the maintenance interval
- **THEN** the service SHALL append the entries without repeating a directory-wide retention scan for each entry

### Requirement: Configured native diagnostic directory
Normal native diagnostics SHALL use the configured active log directory through the unified logging service.

#### Scenario: Persist native diagnostic after settings initialization
- **WHEN** a native diagnostic is emitted after application settings initialize or the user changes the log directory
- **THEN** the service SHALL redact and persist it in the configured active log directory

#### Scenario: Startup fallback before settings are available
- **WHEN** settings cannot yet be loaded during application startup
- **THEN** the service SHALL use the VaneHub application-data fallback directory
- **AND** it SHALL NOT emit the raw diagnostic message to stderr
