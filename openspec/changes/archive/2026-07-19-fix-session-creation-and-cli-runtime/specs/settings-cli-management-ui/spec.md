## ADDED Requirements

### Requirement: CLI management uses branded CLI identity

The CLI management settings page SHALL show the branded icon for each managed CLI.

#### Scenario: CLI cards show tool icons

- **WHEN** the CLI management page lists Claude Code, Codex CLI, Gemini CLI, or OpenCode
- **THEN** each tool card SHALL render that tool's branded icon from the stable agent id.
