# agent-tool-registry Specification

## Purpose
TBD - created by archiving change unify-ai-agent-tool-management. Update Purpose after archive.
## Requirements
### Requirement: Registered agent catalog
The system SHALL maintain a catalog of supported AI coding agents with stable identifiers, display names, provider names, launch metadata, supported interaction modes, availability state, and capability tags.

#### Scenario: Display registered agents
- **WHEN** a user opens the agent selection surface
- **THEN** the system lists each registered agent with its name, provider, availability state, and supported interaction modes

#### Scenario: Preserve stable agent identifiers
- **WHEN** an agent is displayed, selected, or referenced by saved configuration
- **THEN** the system uses the agent's stable identifier instead of relying on display text

### Requirement: Agent availability status
The system SHALL report whether each registered agent is available before the user starts a workflow.

#### Scenario: Agent is available
- **WHEN** a registered agent passes its availability check
- **THEN** the system marks the agent as selectable

#### Scenario: Agent is unavailable
- **WHEN** a registered agent fails its availability check
- **THEN** the system marks the agent as unavailable and provides a reason suitable for user display

### Requirement: Agent capability metadata
The system SHALL associate each registered agent with capability metadata that can be used for filtering, comparison, and future routing decisions.

#### Scenario: Filter by capability
- **WHEN** a user or workflow requests agents with a specific capability tag
- **THEN** the system returns only registered agents that declare that capability tag

### Requirement: SDK-backed agent readiness
The system SHALL be able to use managed SDK dependency status as a readiness signal for agents whose workflows require a managed SDK.

#### Scenario: SDK-backed agent dependency installed
- **WHEN** an agent declares a dependency on a managed SDK and that SDK is installed
- **THEN** the system SHALL allow the agent availability check to treat the managed SDK dependency as satisfied

#### Scenario: SDK-backed agent dependency missing
- **WHEN** an agent declares a dependency on a managed SDK and that SDK is not installed
- **THEN** the system SHALL mark the agent as unavailable or partially unavailable with a reason that identifies the missing SDK dependency

#### Scenario: SDK readiness check does not launch
- **WHEN** the system checks whether a managed SDK dependency is installed for agent availability
- **THEN** the system SHALL NOT launch an interactive agent session
