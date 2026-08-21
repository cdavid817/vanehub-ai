## ADDED Requirements

### Requirement: Claude Code launches declare managed hook ownership
The system SHALL inject an explicit managed permission-hook scope into Claude Code chat and interactive terminal processes launched by VaneHub. The scope SHALL be inherited by Claude Code hook subprocesses and SHALL NOT be injected into other managed CLIs or processes launched independently of VaneHub.

#### Scenario: VaneHub launches Claude Code chat
- **WHEN** VaneHub starts a chat process for stable Agent id `claude-code`
- **THEN** the process environment SHALL declare the managed permission-hook scope

#### Scenario: VaneHub launches a Claude Code terminal
- **WHEN** VaneHub starts an interactive terminal process for stable Agent id `claude-code`
- **THEN** the generated terminal launch environment SHALL declare the managed permission-hook scope

#### Scenario: VaneHub launches another managed CLI
- **WHEN** VaneHub starts Codex CLI, Gemini CLI, OpenCode, or Antigravity CLI
- **THEN** the Claude Code managed permission-hook scope SHALL NOT be present in that process environment
- **AND** the CLI's existing launch-time permission projection SHALL remain unchanged
