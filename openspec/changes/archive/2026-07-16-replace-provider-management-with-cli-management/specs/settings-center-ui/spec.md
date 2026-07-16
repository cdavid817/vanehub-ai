## MODIFIED Requirements

### Requirement: UCD settings pages
The system SHALL provide settings pages for basic configuration, CLI management, SDK dependencies, MCP servers, agents, and skills.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include entries for basic configuration, CLI management, SDK dependencies, MCP servers, agents, and skills

#### Scenario: Display pages without backend services
- **WHEN** a user opens a settings page that does not yet have a dedicated frontend service boundary
- **THEN** the system SHALL render that page using frontend-local data without calling Tauri commands directly from React components

### Requirement: Independent settings page scrolling
Each settings page SHALL scroll within its own content region without moving the settings top navigation or left menu.

#### Scenario: Scroll long settings page content
- **WHEN** Basic Configuration, CLI Management, SDK Dependencies, MCP Servers, Agents, or Skills content exceeds the visible settings content area
- **THEN** the active page SHALL scroll internally while the settings top navigation and left menu remain fixed in place

## ADDED Requirements

### Requirement: CLI management page
The settings center SHALL replace the provider management page with a `CLI 管理` page for supported local AI coding CLI tools.

#### Scenario: Open CLI management page
- **WHEN** a user opens the CLI management settings page
- **THEN** the page SHALL display Anthropic Claude Code CLI, OpenAI Codex CLI, Google Gemini CLI, and OpenCode CLI in that fixed order
- **AND** the page SHALL use service-backed CLI status data rather than frontend-local provider demo data

#### Scenario: Render CLI summary
- **WHEN** the CLI management page renders
- **THEN** it SHALL show only CLI installed and CLI not installed summary counts
- **AND** it SHALL NOT show active provider count, add provider actions, or provider configuration empty states

#### Scenario: Remove provider configuration controls
- **WHEN** the CLI management page renders any CLI card
- **THEN** it SHALL NOT show API Key, URL, preset, enable, edit, or delete controls

### Requirement: Cached CLI status initial rendering
The CLI management page SHALL synchronously read the last persisted CLI detection result for initial rendering without starting expensive detection work.

#### Scenario: Initial page load reads cached result
- **WHEN** a user opens the CLI management page
- **THEN** the page SHALL request the last known CLI status through the frontend service boundary
- **AND** the request SHALL NOT trigger local executable checks, CLI version commands, npm registry queries, install, upgrade, or downgrade commands

#### Scenario: No previous detection
- **WHEN** no persisted detection result exists for a supported CLI
- **THEN** the CLI card SHALL display an undetected state and allow the user to start refresh detection

#### Scenario: First startup auto refresh
- **WHEN** the application starts and no persisted detection result exists for any supported CLI
- **THEN** the system SHALL start one asynchronous CLI detection refresh after reading cached status
- **AND** the startup and settings shell rendering SHALL NOT block on local executable checks, CLI version commands, npm registry queries, install, upgrade, or downgrade commands

### Requirement: CLI detection refresh interaction
The CLI management page SHALL refresh CLI detection and remote version metadata through asynchronous backend-managed operations.

#### Scenario: Start refresh detection
- **WHEN** the user activates the refresh detection action
- **THEN** the page SHALL start an asynchronous refresh operation through the frontend service boundary
- **AND** the settings shell SHALL remain interactive while the operation runs

#### Scenario: Display refreshed CLI metadata
- **WHEN** refresh detection completes for a supported CLI
- **THEN** the corresponding card SHALL display installed state, current version, latest version, local install path, available versions, last checked time, or a user-displayable per-CLI error

#### Scenario: One CLI refresh fails
- **WHEN** refresh detection fails for one supported CLI but succeeds for another
- **THEN** the page SHALL preserve and display the successful CLI result and show the failed CLI's error without failing the whole page

### Requirement: CLI version actions
The CLI management page SHALL allow installing, upgrading, or downgrading supported CLI tools by selecting a target stable version.

#### Scenario: Stable version selection
- **WHEN** available versions are displayed for a CLI
- **THEN** the page SHALL show at most the latest 20 stable versions by default
- **AND** it SHALL exclude prerelease versions

#### Scenario: Install missing CLI
- **WHEN** a CLI is not installed and the user selects a target version
- **THEN** the page SHALL present an install action for that version

#### Scenario: Upgrade installed CLI
- **WHEN** a CLI is installed and the selected target version is newer than the current version
- **THEN** the page SHALL present an upgrade action for that version

#### Scenario: Downgrade installed CLI
- **WHEN** a CLI is installed and the selected target version is older than the current version
- **THEN** the page SHALL present a downgrade action for that version

#### Scenario: Current CLI version selected
- **WHEN** a CLI is installed and the selected target version equals the current version
- **THEN** the page SHALL present the current-version state and prevent a redundant package operation

### Requirement: CLI operation feedback
The CLI management page SHALL show the most recent operation state and expandable logs inside each affected CLI card.

#### Scenario: Operation state in CLI card
- **WHEN** a refresh, install, upgrade, or downgrade operation is associated with a CLI
- **THEN** that CLI card SHALL show the latest operation status without requiring a global log panel

#### Scenario: Expand operation logs
- **WHEN** the user expands operation details for a CLI card
- **THEN** the page SHALL display the logs associated with that CLI's most recent operation

#### Scenario: Card-local disabled controls
- **WHEN** a CLI operation is running
- **THEN** the page SHALL disable only controls affected by that operation and SHALL keep unrelated CLI cards and settings navigation interactive
