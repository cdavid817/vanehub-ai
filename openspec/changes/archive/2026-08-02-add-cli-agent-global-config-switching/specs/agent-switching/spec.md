## ADDED Requirements

### Requirement: Global CLI configuration activation is independent from runtime selection
The system SHALL treat applying a user-level CLI configuration profile as a configuration-management operation distinct from selecting an active Agent, interaction mode, or Session workflow.

#### Scenario: Apply a profile while a Session is active
- **WHEN** a user applies a Claude Code, OpenCode, or Codex CLI global configuration profile while any Session is active
- **THEN** the system SHALL leave `workflow_state`, the active Session id, the Session's Agent id, and its interaction mode unchanged
- **AND** SHALL report that an already-running CLI process may require restart rather than silently restarting or rerouting it

#### Scenario: Select runtime Agent after applying a profile
- **WHEN** a user later selects that registered Agent for a new or existing compatible workflow
- **THEN** the existing stable-Agent-id and interaction-mode validation SHALL continue to apply independently of which global configuration profile is recorded as applied
