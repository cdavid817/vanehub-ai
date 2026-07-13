## ADDED Requirements

### Requirement: Active agent selection
The system SHALL allow users to select one available AI coding agent as the active agent for a development workflow.

#### Scenario: Select available agent
- **WHEN** a user selects an available registered agent
- **THEN** the system records that agent as the active agent for the current workflow

#### Scenario: Prevent unavailable agent selection
- **WHEN** a user attempts to select an unavailable registered agent
- **THEN** the system prevents the selection and shows why the agent is unavailable

### Requirement: Compatible mode selection
The system SHALL require the active interaction mode to be compatible with the selected agent.

#### Scenario: Select supported interaction mode
- **WHEN** a user selects an interaction mode supported by the active agent
- **THEN** the system records that mode for the current workflow

#### Scenario: Reject unsupported interaction mode
- **WHEN** a user selects an interaction mode that the active agent does not support
- **THEN** the system rejects the mode and shows the supported modes for that agent

### Requirement: Agent switch preserves workflow intent
The system SHALL preserve the current workflow intent when switching between compatible agents.

#### Scenario: Switch active agent
- **WHEN** a user switches from one available agent to another available agent
- **THEN** the system keeps the current workflow context and updates launch routing to the newly selected agent

#### Scenario: Switch requires mode update
- **WHEN** a user switches to an agent that does not support the current interaction mode
- **THEN** the system requires the user to choose a supported interaction mode before continuing
