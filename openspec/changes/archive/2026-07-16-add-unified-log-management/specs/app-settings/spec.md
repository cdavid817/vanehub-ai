## ADDED Requirements

### Requirement: Logging settings model
The system SHALL include log directory and read-only logging policy values in the shared settings model.

#### Scenario: Load default logging settings
- **WHEN** no persisted logging settings exist
- **THEN** the system SHALL provide a valid default log directory and fixed first-version policies for 30-day retention, automatic archival, built-in redaction, and supported log levels

#### Scenario: Save log directory setting
- **WHEN** a user saves a log directory in the desktop runtime
- **THEN** the system SHALL persist the directory through the settings service and use it for newly written logs

#### Scenario: Reject invalid log directory
- **WHEN** the runtime cannot validate or create the requested log directory
- **THEN** the system SHALL reject the setting without changing the active log directory

#### Scenario: Restore log directory setting
- **WHEN** the application restarts after a custom log directory has been saved
- **THEN** the system SHALL restore that directory as the active log directory
