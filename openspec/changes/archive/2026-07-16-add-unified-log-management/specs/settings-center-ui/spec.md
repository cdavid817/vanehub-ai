## ADDED Requirements

### Requirement: Basic Settings log management section
The Basic Settings page SHALL provide a log management section for the active runtime.

#### Scenario: Display desktop log directory
- **WHEN** the Basic Settings page loads in the Tauri desktop runtime
- **THEN** it SHALL display the active log directory from the settings service

#### Scenario: Change desktop log directory
- **WHEN** a user changes the log directory from Basic Settings
- **THEN** the page SHALL save the directory through the settings service without calling Tauri APIs directly

#### Scenario: Open desktop log directory
- **WHEN** a user selects the open log directory action in the Tauri desktop runtime
- **THEN** the page SHALL request the action through the settings service

#### Scenario: Display logging policies
- **WHEN** the Basic Settings page displays log management
- **THEN** it SHALL show that retention is fixed at 30 days, archival is automatic, redaction is built in, and supported log levels are `error`, `warn`, `info`, and `debug`

#### Scenario: Disable native open action in Web runtime
- **WHEN** the Basic Settings page runs with the Web/mock adapter
- **THEN** it SHALL display the mock log path and keep the open log directory action disabled
