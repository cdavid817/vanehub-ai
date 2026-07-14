## ADDED Requirements

### Requirement: Agent switching in settings center
The system SHALL preserve agent switching behavior when agent management is presented inside the UCD-aligned Agents settings page.

#### Scenario: Select agent by stable id from Agents page
- **WHEN** a user selects an available agent from the Agents settings page
- **THEN** the system SHALL call the agent service with the selected stable agent id rather than matching by display name

#### Scenario: Select compatible interaction mode from Agents page
- **WHEN** a user selects an interaction mode for an agent from the Agents settings page
- **THEN** the system SHALL preserve the existing compatible mode validation and show supported modes when the selected mode is unsupported

#### Scenario: Launch selected workflow from Agents page
- **WHEN** a user launches the active workflow from the Agents settings page
- **THEN** the system SHALL use the existing agent service launch flow and MUST keep availability and browser readiness checks separate from launching an interactive session

### Requirement: Agent status visibility in settings center
The system SHALL show agent availability, supported interaction modes, capability tags, active selection, workflow lifecycle, and session details in the UCD-aligned Agents settings page.

#### Scenario: Display agent registry state
- **WHEN** the Agents settings page loads registered agents
- **THEN** the system SHALL display each agent's provider, availability state, supported interaction modes, and capability tags from the agent service response

#### Scenario: Display current workflow state
- **WHEN** the current workflow has an active agent or lifecycle state
- **THEN** the system SHALL display the active agent, active interaction mode, lifecycle state, and session details in the Agents settings page
