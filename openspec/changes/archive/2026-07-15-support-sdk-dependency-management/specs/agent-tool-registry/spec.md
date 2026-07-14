## ADDED Requirements

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
