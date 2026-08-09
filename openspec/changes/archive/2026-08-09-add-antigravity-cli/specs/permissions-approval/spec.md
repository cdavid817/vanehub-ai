## MODIFIED Requirements

### Requirement: Agent policy list surfaces every eligible agent's current template
The system SHALL provide a settings surface listing every custom API agent, the built-in OnePiece agent, and the five stable managed CLI principals (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`, `antigravity-cli`), each showing its currently assigned policy template, without requiring the user to inspect storage directly.

#### Scenario: Custom agents and OnePiece appear in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display every agent with `agentOrigin` of `user`, plus the OnePiece agent, each with its current policy template

#### Scenario: An agent with no explicit assignment shows the effective default
- **WHEN** a listed agent has never been assigned a policy template
- **THEN** the system SHALL display the current default template as its effective template, rather than an empty or unknown state

#### Scenario: The claude-code CLI principal appears in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display the `claude-code` principal alongside custom agents and OnePiece, with its current policy template or effective default

#### Scenario: The codex-cli, gemini-cli, and opencode CLI principals appear in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display the `codex-cli`, `gemini-cli`, and `opencode` principals alongside `claude-code`, custom agents, and OnePiece, each with its current policy template or effective default

#### Scenario: The antigravity-cli principal appears in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display the `antigravity-cli` principal alongside the other managed CLI principals, custom agents, and OnePiece, with its current policy template or effective default
