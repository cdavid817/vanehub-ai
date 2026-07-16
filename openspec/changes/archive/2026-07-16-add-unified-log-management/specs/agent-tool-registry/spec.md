## ADDED Requirements

### Requirement: CLI unified operation log persistence
CLI detection and package operations SHALL persist operation logs through unified log management.

#### Scenario: Persist CLI detection logs
- **WHEN** a CLI detection or remote version refresh operation emits diagnostic or operation output
- **THEN** the system SHALL write the redacted output to the active log directory with agent id and operation context

#### Scenario: Persist CLI package logs
- **WHEN** a CLI install, upgrade, or downgrade operation emits stdout, stderr, completion, or failure output
- **THEN** the system SHALL write the redacted output to the active log directory with agent id and operation context

#### Scenario: Keep CLI card logs
- **WHEN** CLI operation logs are persisted through unified log management
- **THEN** the CLI management page SHALL still display the latest operation logs inside the affected CLI card
