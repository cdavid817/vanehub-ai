## MODIFIED Requirements

### Requirement: Prompt Hook CLI bindings
The system SHALL bind Prompt Hooks to supported CLI agents by stable agent id.

#### Scenario: Bind hook to CLI agents
- **WHEN** a user updates a Prompt Hook's CLI bindings
- **THEN** the service SHALL persist only supported stable CLI agent ids among `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`
- **AND** it SHALL NOT match agents by display name

#### Scenario: Unbound hook does not apply
- **WHEN** a Prompt Hook has no binding for the active session's stable CLI agent id
- **THEN** the Prompt Hook pipeline SHALL skip that hook for the invocation
