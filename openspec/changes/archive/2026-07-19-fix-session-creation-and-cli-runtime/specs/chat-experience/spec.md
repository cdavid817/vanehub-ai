## ADDED Requirements

### Requirement: Session agent identity

The system SHALL show the selected agent/CLI icon on the session page.

#### Scenario: Codex session shows Codex identity

- **WHEN** a session is created for Codex CLI
- **THEN** the session page SHALL display the Codex CLI icon or registered visual identity alongside the session metadata.

#### Scenario: Session surfaces use stable CLI identity

- **WHEN** a session references Claude Code, Codex CLI, Gemini CLI, or OpenCode
- **THEN** session list and detail surfaces SHALL render the corresponding branded CLI icon from the stable agent id.
