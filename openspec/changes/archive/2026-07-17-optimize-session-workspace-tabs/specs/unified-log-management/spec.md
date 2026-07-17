## ADDED Requirements

### Requirement: Session diagnostics query
The system SHALL expose redacted session-scoped diagnostics from the active unified log through the frontend service boundary without creating a second SQLite log store.

#### Scenario: Query logs for a session
- **WHEN** the frontend requests logs for a valid session id
- **THEN** the desktop runtime SHALL read the active unified log and return only redacted entries whose structured context contains that session id

#### Scenario: Filter session logs
- **WHEN** the request includes log levels or search text
- **THEN** the desktop runtime SHALL apply the level filter and case-insensitive search before returning a bounded result page

#### Scenario: Ignore malformed log lines safely
- **WHEN** the unified log contains a malformed entry
- **THEN** the desktop runtime SHALL skip that entry without returning its raw content to the frontend

#### Scenario: Query logs in Web runtime
- **WHEN** the Web/mock adapter receives a session log query
- **THEN** it SHALL return deterministic redacted mock entries and SHALL NOT read a local file

### Requirement: Session diagnostics export
The system SHALL allow desktop users to export filtered, already-redacted session diagnostics through a declared backend operation.

#### Scenario: Export filtered session logs
- **WHEN** a desktop user chooses a destination and confirms a session log export
- **THEN** the native runtime SHALL write only the matching redacted entries to that destination and return a concise success result

#### Scenario: Cancel session log export
- **WHEN** the user cancels destination selection
- **THEN** the system SHALL leave existing files unchanged and report the cancellation without an error

#### Scenario: Prevent arbitrary source export
- **WHEN** the frontend requests a session log export
- **THEN** the frontend SHALL provide filter criteria rather than an arbitrary native source path

#### Scenario: Export unavailable in Web runtime
- **WHEN** the user requests local session log export in Web/mock mode
- **THEN** the service SHALL report that local export is unavailable and SHALL NOT claim that a file was written

