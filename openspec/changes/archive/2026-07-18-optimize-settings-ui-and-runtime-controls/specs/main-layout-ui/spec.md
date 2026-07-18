## ADDED Requirements

### Requirement: CLI-specific session icons
The workspace shell SHALL render CLI-specific visual identity for sessions based on each session's stable agent id.

#### Scenario: Render session card CLI icon
- **WHEN** a session card renders for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the card SHALL show the corresponding CLI icon or semantic icon treatment for that stable agent id
- **AND** the icon SHALL remain visually distinct from the other managed CLI tools

#### Scenario: Render created session with selected CLI icon
- **WHEN** the user creates a session from the create-session dialog for a selected CLI
- **THEN** the created session SHALL appear in workspace navigation with that selected CLI's icon identity

#### Scenario: Fallback unknown agent icon
- **WHEN** a session references an unknown or future agent id
- **THEN** the workspace SHALL render a neutral fallback agent icon without failing the session list

#### Scenario: Preserve compact session layout
- **WHEN** CLI-specific icons render in session cards, active-session headers, or session-adjacent workspace surfaces
- **THEN** long titles, folder paths, status markers, and context actions SHALL not overlap the icon or each other
