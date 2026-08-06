## MODIFIED Requirements

### Requirement: Unified permission decision model
The system SHALL evaluate every gated action, whether requested by a native API agent's tool-use loop or forwarded through the Claude Code permission-hook bridge, through a single decision point that resolves a `(principal, action, resource)` triple to exactly one of `Allow`, `Deny`, or `Ask`. A principal SHALL be identified by a stable agent id alone — one durable principal per agent, persisting across every session that agent participates in — with session id and generation id carried as per-evaluation context rather than as part of the principal's own identity.

#### Scenario: Evaluation produces one of three effects
- **WHEN** the native agent's tool-use loop requests an action requiring a permission decision
- **THEN** the system SHALL resolve it to exactly one of `Allow`, `Deny`, or `Ask` before the tool executes

#### Scenario: Unmatched action defaults to Ask
- **WHEN** no policy matches the requested principal, action, and resource
- **THEN** the system SHALL resolve the evaluation to `Ask`, not `Allow`

#### Scenario: The same principal is used across every session for an agent
- **WHEN** an agent participates in a new session it has never used before
- **THEN** the system SHALL evaluate that agent's actions against the same principal and policy assignment used in its other sessions, not a new, session-scoped principal

#### Scenario: CLI-originated evaluation uses the same decision point
- **WHEN** the Claude Code permission-hook bridge forwards a mapped tool call for the `claude-code` principal
- **THEN** the system SHALL resolve it through the same decision point, policy templates, and grants a native agent's equivalent action would use
