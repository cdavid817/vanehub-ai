## ADDED Requirements

### Requirement: Agent policy list surfaces every eligible agent's current template
The system SHALL provide a settings surface listing every custom API agent and the built-in OnePiece agent, each showing its currently assigned policy template, without requiring the user to inspect storage directly.

#### Scenario: Custom agents and OnePiece appear in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display every agent with `agentOrigin` of `user`, plus the OnePiece agent, each with its current policy template

#### Scenario: An agent with no explicit assignment shows the effective default
- **WHEN** a listed agent has never been assigned a policy template
- **THEN** the system SHALL display the current default template as its effective template, rather than an empty or unknown state

### Requirement: Reading a principal's policy template never creates it
The system SHALL be able to report an agent principal's current policy template without creating a stored principal record as a side effect of that read.

#### Scenario: Listing agents does not write principal rows
- **WHEN** the agent policy settings surface lists agents that have never been evaluated or explicitly assigned a template
- **THEN** the system SHALL NOT create a stored principal record for any of them as a result of that listing
