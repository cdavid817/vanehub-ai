# agent-switching Specification

## Purpose
TBD - created by archiving change unify-ai-agent-tool-management. Update Purpose after archive.
## Requirements
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

### Requirement: Observable agent launch operations
Agent launch flows that may start external processes, open browser workflows, or initialize native desktop sessions SHALL expose observable operation state when launch cannot complete as a short immediate command.

#### Scenario: Launch starts observable operation
- **WHEN** a user launches an active workflow and the launch path requires a long-running or externally visible operation
- **THEN** the system SHALL expose an operation id, lifecycle state, and user-displayable status through the agent service boundary

#### Scenario: Launch readiness remains separate
- **WHEN** the system checks Agent availability or browser readiness before launch
- **THEN** it SHALL perform those checks separately from starting the observable launch operation
