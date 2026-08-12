## MODIFIED Requirements

### Requirement: CLI management page
The settings center SHALL replace the provider management page with a `CLI 管理` page for supported local AI coding CLI tools.

#### Scenario: Open CLI management page
- **WHEN** a user opens the CLI management settings page
- **THEN** the page SHALL display Anthropic Claude Code CLI, OpenAI Codex CLI, OpenCode CLI, Antigravity CLI, and Google Gemini CLI in that fixed order
- **AND** the page SHALL use service-backed CLI status data rather than frontend-local provider demo data

#### Scenario: Render CLI summary
- **WHEN** the CLI management page renders
- **THEN** it SHALL show only CLI installed and CLI not installed summary counts
- **AND** it SHALL NOT show active provider count, add provider actions, or provider configuration empty states

#### Scenario: Remove provider configuration controls
- **WHEN** the CLI management page renders any CLI card
- **THEN** it SHALL NOT show API Key, URL, preset, enable, edit, or delete controls

