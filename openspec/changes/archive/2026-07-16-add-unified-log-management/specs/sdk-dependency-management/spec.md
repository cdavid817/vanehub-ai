## ADDED Requirements

### Requirement: SDK unified operation log persistence
SDK dependency operations SHALL persist operation logs through unified log management.

#### Scenario: Persist SDK install logs
- **WHEN** an SDK install operation emits log output
- **THEN** the system SHALL write the redacted output to the active log directory with SDK id and operation context

#### Scenario: Persist SDK update rollback and uninstall logs
- **WHEN** an SDK update, rollback, or uninstall operation emits log output
- **THEN** the system SHALL write the redacted output to the active log directory with SDK id and operation context

#### Scenario: Keep SDK settings page logs
- **WHEN** SDK operation logs are persisted through unified log management
- **THEN** the SDK settings page SHALL still be able to display operation logs through the SDK frontend service
