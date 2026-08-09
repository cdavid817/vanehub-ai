## MODIFIED Requirements

### Requirement: CLI-specific session icons
The workspace shell SHALL render CLI-specific visual identity for sessions based on each session's stable agent id.

#### Scenario: Render session card CLI icon
- **WHEN** a session card renders for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli`
- **THEN** the card SHALL show the corresponding CLI icon or semantic icon treatment for that stable agent id
- **AND** the icon SHALL remain visually distinct from the other managed CLI tools

#### Scenario: Render created session with selected CLI icon
- **WHEN** the user creates a session from the create-session dialog for a selected CLI
- **THEN** the created session SHALL appear in workspace navigation with that selected CLI's icon identity
