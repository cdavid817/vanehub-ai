## ADDED Requirements

### Requirement: Derived session visual identity
The system SHALL derive session icon identity from the session's stable agent id rather than persisting redundant icon metadata in the session entity.

#### Scenario: Store stable agent id only
- **WHEN** a session is created for Claude Code, Gemini CLI, Codex CLI, or OpenCode
- **THEN** the session record SHALL store the selected stable agent id
- **AND** it SHALL NOT require a persisted icon name, icon path, or icon color field

#### Scenario: Derive icon after reload
- **WHEN** persisted sessions are listed after app restart or Web/mock reload
- **THEN** the UI SHALL be able to render the CLI-specific icon from the stored stable agent id
