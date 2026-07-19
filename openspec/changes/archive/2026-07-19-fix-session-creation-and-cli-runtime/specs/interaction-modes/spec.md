## ADDED Requirements

### Requirement: Create-session mode selection

The system SHALL keep implementation interaction modes internal during session creation and SHALL NOT present raw `cli` or `native-desktop` mode identifiers as primary user-facing choices.

#### Scenario: User creates a session

- **WHEN** the user opens the create-session page
- **THEN** the page SHALL present user-oriented choices such as agent and local/remote workspace
- **AND** it SHALL NOT present raw `cli` or `native-desktop` options as standalone choices.

#### Scenario: Selected CLI is explicit

- **WHEN** the user selects a CLI agent during session creation
- **THEN** the selected CLI SHALL have a visually distinct selected state
- **AND** the page SHALL show a clear textual indication of the currently selected CLI.
