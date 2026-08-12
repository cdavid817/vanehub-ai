## ADDED Requirements

### Requirement: OnePiece scheduled-task execution
Scheduled Tasks SHALL treat the native OnePiece Agent as an eligible automation target in addition to supported CLI Agents.

#### Scenario: Create a OnePiece scheduled task
- **WHEN** the user creates a scheduled task with stable Agent id `onepiece`
- **THEN** the system SHALL accept the task when OnePiece is registered and available
- **AND** the scheduled runner SHALL start a OnePiece API interaction rather than a CLI terminal interaction

#### Scenario: Reject another non-CLI Agent
- **WHEN** a scheduled task references a non-CLI Agent other than `onepiece`
- **THEN** the system SHALL reject the task with a validation error before persistence

