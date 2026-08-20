## MODIFIED Requirements

### Requirement: SDK-backed agent readiness
The system SHALL be able to use managed SDK dependency status as a readiness signal for agents whose workflows require a managed SDK, but SHALL NOT let that status contradict a working installation.

An agent whose declared executable resolves on the host SHALL be reported available regardless of its managed SDK status. Managed SDK status SHALL decide availability only for an agent that declares no executable, where it is the only evidence there is. A managed SDK is a package the product offers to install and roll back; nothing on the execution path loads it, because managed agents are driven through their command-line interface.

When an agent is unavailable, the reported reason SHALL name what the user has to act on: a missing executable SHALL be reported against the search path, not against the SDK.

#### Scenario: SDK-backed agent dependency installed
- **WHEN** an agent declares a dependency on a managed SDK and that SDK is installed
- **THEN** the system SHALL allow the agent availability check to treat the managed SDK dependency as satisfied

#### Scenario: Executable present while the managed SDK is missing
- **WHEN** an agent declares a managed SDK dependency that is not installed
- **AND** the agent's declared executable resolves on the host search path
- **THEN** the system SHALL report the agent available
- **AND** it SHALL report no unavailability reason
- **AND** the agent SHALL be selectable for a session

#### Scenario: Executable missing while the managed SDK is missing
- **WHEN** an agent declares a managed SDK dependency that is not installed
- **AND** the agent's declared executable does not resolve on the host search path
- **THEN** the system SHALL mark the agent unavailable with a reason identifying the missing executable

#### Scenario: SDK-backed agent dependency missing
- **WHEN** an agent declares a dependency on a managed SDK and that SDK is not installed
- **AND** the agent declares no executable to probe, so the SDK is the only evidence available
- **THEN** the system SHALL mark the agent as unavailable or partially unavailable with a reason that identifies the missing SDK dependency

#### Scenario: SDK readiness check does not launch
- **WHEN** the system checks whether a managed SDK dependency is installed for agent availability
- **THEN** the system SHALL NOT launch an interactive agent session
