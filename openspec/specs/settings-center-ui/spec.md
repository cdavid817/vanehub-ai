# settings-center-ui Specification

## Purpose
Defines the VaneHub AI settings-center shell, UCD-aligned settings pages, and switchable visual style behavior shared by the Tauri desktop frontend and browser Web runtime.
## Requirements
### Requirement: Settings center shell
The system SHALL render a UCD-aligned settings center as the primary frontend surface with top navigation, settings sidebar navigation, and a page content area.

#### Scenario: Render settings center
- **WHEN** a user opens the VaneHub AI frontend in the Tauri desktop runtime or browser Web runtime
- **THEN** the system SHALL show the settings center shell with VaneHub AI branding, settings navigation, and a selected settings page

#### Scenario: Navigate settings pages
- **WHEN** a user selects a settings navigation item
- **THEN** the system SHALL update the active page content and active navigation state without requiring a runtime-specific backend call

### Requirement: UCD settings pages
The system SHALL provide settings pages for basic configuration, CLI management, SDK dependencies, MCP servers, agents, and skills.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include entries for basic configuration, CLI management, SDK dependencies, MCP servers, agents, and skills

#### Scenario: Display pages without backend services
- **WHEN** a user opens a settings page that does not yet have a dedicated frontend service boundary
- **THEN** the system SHALL render that page using frontend-local data without calling Tauri commands directly from React components

### Requirement: Switchable UCD visual styles
The system SHALL support switching between the `futuristic` and `minimal` UCD visual styles through a shared theme mechanism.

#### Scenario: Switch visual style
- **WHEN** a user selects a different UCD visual style
- **THEN** the system SHALL update the settings center appearance while preserving the current active settings page and page state

#### Scenario: Apply style consistently
- **WHEN** a UCD visual style is active
- **THEN** the system SHALL apply that style consistently to the top navigation, sidebar, content panels, controls, badges, and detail areas through semantic design tokens

### Requirement: Extensible style registration
The system SHALL register visual styles through a central frontend registry so future styles can be added without modifying page-specific business logic.

#### Scenario: Registered styles populate switcher
- **WHEN** the style switcher is rendered
- **THEN** the system SHALL derive available style options from the central style registry

#### Scenario: Future style addition
- **WHEN** a developer adds a new registered style and matching semantic token definitions
- **THEN** the system SHALL make that style available without requiring conditional style branches inside each settings page component

### Requirement: Style persistence
The system SHALL persist the selected UCD visual style in frontend-local storage for both browser Web and Tauri desktop runtimes.

#### Scenario: Restore selected style
- **WHEN** a user selects a UCD visual style and later reopens the frontend
- **THEN** the system SHALL restore the last valid selected style

#### Scenario: Invalid persisted style
- **WHEN** the persisted style value does not match a registered style
- **THEN** the system SHALL fall back to the default registered style

### Requirement: Stateful settings page mounting
The system SHALL preserve mounted state for settings pages that maintain runtime-local UI state across page navigation.

#### Scenario: Preserve settings page state
- **WHEN** a user navigates away from a stateful settings page and later returns to it
- **THEN** the system SHALL show the page with its local UI state preserved instead of remounting it from scratch

### Requirement: Service-backed MCP settings page
The system SHALL render the MCP settings page as a service-backed management surface rather than a static demo data page.

#### Scenario: Display MCP server configurations
- **WHEN** a user opens the MCP settings page
- **THEN** the page SHALL load MCP server configurations through the MCP frontend service interface

#### Scenario: Manage MCP servers from settings
- **WHEN** a user adds, edits, renames, deletes, toggles, tests, imports, or exports MCP servers from the settings page
- **THEN** the page SHALL perform those operations through the MCP frontend service interface

#### Scenario: Empty MCP state
- **WHEN** no MCP servers are visible for the current user and project scopes
- **THEN** the page SHALL show an empty state with an action to add the first MCP server

### Requirement: Service-backed SDK settings page
The system SHALL render the SDK dependencies page as a service-backed management surface rather than a static demo data page.

#### Scenario: Display SDK dependency statuses
- **WHEN** a user opens the SDK dependencies settings page
- **THEN** the page SHALL load managed SDK dependency statuses through the SDK frontend service interface

#### Scenario: Manage SDK dependencies from settings
- **WHEN** a user refreshes, checks versions, installs, updates, rolls back, or uninstalls an SDK dependency from the settings page
- **THEN** the page SHALL perform those operations through the SDK frontend service interface

#### Scenario: Display SDK operation logs
- **WHEN** an SDK install, update, rollback, or uninstall operation produces logs
- **THEN** the SDK settings page SHALL display those logs in the page while preserving the selected SDK page state

#### Scenario: Preserve settings page style
- **WHEN** the SDK dependencies page renders service-backed data and controls
- **THEN** the page SHALL use the shared settings center layout, semantic design tokens, controls, and status styles consistently with the rest of the settings center

### Requirement: SDK version action controls
The system SHALL present selectable SDK versions and derive the primary action from installed state and selected target version.

#### Scenario: Install action for missing SDK
- **WHEN** an SDK is not installed and a target version is selected
- **THEN** the page SHALL present an install action for that target version

#### Scenario: Update action for newer version
- **WHEN** an SDK is installed and the selected target version is newer than the installed version
- **THEN** the page SHALL present an update action for that target version

#### Scenario: Rollback action for older version
- **WHEN** an SDK is installed and the selected target version is older than the installed version
- **THEN** the page SHALL present a rollback action for that target version

#### Scenario: Current version action disabled
- **WHEN** an SDK is installed and the selected target version equals the installed version
- **THEN** the page SHALL present the current-version state and prevent a redundant install operation

### Requirement: Shared settings data orchestration
Settings pages that load or mutate service-backed data SHALL use the shared frontend data-fetching foundation for request state, cache invalidation, refresh, loading, and error behavior.

#### Scenario: Refresh service-backed settings page
- **WHEN** a user refreshes a service-backed settings page
- **THEN** the page SHALL perform the refresh through the shared data-fetching foundation and preserve unrelated local UI state

#### Scenario: Settings mutation succeeds
- **WHEN** a settings page mutation succeeds
- **THEN** the page SHALL invalidate or refresh the affected service-backed data through the shared data-fetching foundation

### Requirement: Shared settings form validation
Settings pages that collect configuration input SHALL use shared schema-backed form validation before submitting through service interfaces.

#### Scenario: Invalid settings form
- **WHEN** a user submits invalid MCP, SDK, provider, Agent, or basic settings input
- **THEN** the settings page SHALL show field-level validation errors and SHALL NOT call a backend or runtime adapter for that invalid submission

### Requirement: Unified tool entry from workspace
The settings center SHALL remain reachable from the workspace sidebar utility row and SHALL be the unified destination for the six tool shortcuts removed from the workspace sidebar.

#### Scenario: Open settings from workspace tool entry
- **WHEN** the user activates the workspace Settings utility button
- **THEN** the system SHALL open the settings center without requiring a runtime-specific backend call

#### Scenario: Preserve settings page behavior
- **WHEN** the settings center is opened as the unified tool destination
- **THEN** the settings center SHALL preserve existing navigation, page mounting, visual style, and service boundary behavior

### Requirement: Independent settings page scrolling
Each settings page SHALL scroll within its own content region without moving the settings top navigation or left menu.

#### Scenario: Scroll long settings page content
- **WHEN** Basic Configuration, CLI Management, SDK Dependencies, MCP Servers, Agents, or Skills content exceeds the visible settings content area
- **THEN** the active page SHALL scroll internally while the settings top navigation and left menu remain fixed in place

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

### Requirement: Service-backed basic configuration
The Basic Configuration page SHALL render common application settings through the shared settings provider and frontend service boundary.

#### Scenario: Display common settings controls
- **WHEN** a user opens the Basic Configuration page
- **THEN** the page SHALL display controls for application language, font size, visual theme, default folder path, and read-only Node.js environment information

#### Scenario: Update common setting
- **WHEN** a user changes language, font size, visual theme, or default folder path from the Basic Configuration page
- **THEN** the page SHALL save the setting through the shared settings provider without directly calling a Tauri command

#### Scenario: Preserve settings page layout
- **WHEN** Basic Configuration renders common settings controls
- **THEN** the page SHALL use the shared settings center layout, semantic design tokens, controls, and internal scrolling behavior

### Requirement: Localized settings center text
The settings center SHALL render user-visible text through synchronized zh-CN and en translation resources.

#### Scenario: Render Chinese language
- **WHEN** the active application language is Chinese
- **THEN** settings center pages SHALL render extracted zh-CN translation values instead of hard-coded Chinese literals

#### Scenario: Render English language
- **WHEN** the active application language is English
- **THEN** settings center pages SHALL render corresponding en translation values for the same translation keys

#### Scenario: Translation resources stay aligned
- **WHEN** a translation key is added for settings center or related application surfaces
- **THEN** the zh-CN and en translation resources SHALL contain matching keys

### Requirement: Service-backed Skills settings page
The Skills settings page SHALL render as a service-backed Skill management surface rather than a static demo data page.

#### Scenario: Load Skills settings data
- **WHEN** a user opens the Skills settings page
- **THEN** the page SHALL load Skills, registered Agents, Agent mount paths, Skill statistics, and drift status through the frontend service boundary

#### Scenario: No static demo data
- **WHEN** the Skills settings page renders
- **THEN** the page SHALL NOT use hard-coded demo Skill arrays as the source of displayed Skill data

### Requirement: Skills page module composition
The Skills settings page SHALL be composed from seven reusable child components: `SkillStatsCards`, `SkillAgentMountPathsPanel`, `SkillScopeTabs`, `SkillFilterToolbar`, `SkillCardList`, `SkillDialogs`, and `SkillDriftBanner`.

#### Scenario: Render Skill management modules
- **WHEN** the Skills settings page has loaded data
- **THEN** it SHALL show statistics, Agent mount paths, scope controls, filters, Skill cards, dialogs, drift status, and bottom summary behavior through the composed modules

### Requirement: Skill statistics and summary
The Skills settings page SHALL display core Skill metrics and a bottom summary for the active scope and filters.

#### Scenario: Display Skill statistics
- **WHEN** the page renders loaded Skill data
- **THEN** it SHALL show counts for all Skills, enabled Skills, and mounted Skills

#### Scenario: Display filtered summary
- **WHEN** a user changes scope, category, search query, enabled state, or Agent binding
- **THEN** the bottom summary SHALL reflect the current visible Skill set and active scope

### Requirement: Agent mount path panel
The Skills settings page SHALL show all registered Agents with editable Skill mount paths.

#### Scenario: Display Agent mount paths
- **WHEN** registered Agents are loaded
- **THEN** the page SHALL display each Agent with its current Skill mount path as a code-style label

#### Scenario: Edit Agent mount path
- **WHEN** a user changes an Agent mount path
- **THEN** the page SHALL submit the change through the frontend service boundary and display the migration result returned by the service

### Requirement: Skill scope selection
The Skills settings page SHALL support `global` and `workspace` scope selection.

#### Scenario: Switch to global scope
- **WHEN** a user selects the global scope tab
- **THEN** the page SHALL load global Skills and global drift status

#### Scenario: Select workspace directory
- **WHEN** a user selects the workspace scope
- **THEN** the page SHALL provide a directory picker for choosing the local project directory

#### Scenario: Workspace scope load
- **WHEN** a workspace directory is selected
- **THEN** the page SHALL load Skills and drift status for that workspace directory only

### Requirement: Skill filtering and search
The Skills settings page SHALL allow users to filter Skills by category and search by keyword.

#### Scenario: Category filter
- **WHEN** a user selects a Skill category
- **THEN** the Skill card list SHALL show only Skills in that category

#### Scenario: Keyword search
- **WHEN** a user enters a search query
- **THEN** the Skill card list SHALL match Skills by id, name, description, category, triggers, or source label

### Requirement: Skill card controls
Each Skill card SHALL provide enablement, Agent binding, source labeling, preview, edit, and delete controls.

#### Scenario: Toggle Skill enabled state
- **WHEN** a user toggles a Skill enabled state
- **THEN** the page SHALL submit the change through the frontend service boundary and refresh the affected Skill and drift state

#### Scenario: Toggle Agent binding
- **WHEN** a user changes Agent binding checkboxes on a Skill card
- **THEN** the page SHALL submit the binding set through the frontend service boundary and refresh the affected Skill and drift state

#### Scenario: Source badge
- **WHEN** a Skill card renders
- **THEN** it SHALL display whether the Skill source is built-in, user-created, or imported

### Requirement: Skill dialogs
The Skills settings page SHALL provide dialogs for `SKILL.md` preview, Skill creation, Skill editing, external Skill import, and built-in Skill restore.

#### Scenario: Preview SKILL.md
- **WHEN** a user opens Skill preview
- **THEN** the dialog SHALL display the current `SKILL.md` source content loaded through the frontend service boundary

#### Scenario: Create Skill
- **WHEN** a user submits a valid create Skill form
- **THEN** the page SHALL create a Skill with immutable id and valid `SKILL.md` frontmatter through the frontend service boundary

#### Scenario: Edit Skill
- **WHEN** a user edits a Skill
- **THEN** the form SHALL prevent changing the Skill id and SHALL submit editable metadata and body through the frontend service boundary

#### Scenario: Import external Skill
- **WHEN** a user imports an external Skill directory
- **THEN** the page SHALL call the frontend service boundary to copy it into the selected scope and refresh the Skill list

#### Scenario: Restore built-in Skill
- **WHEN** a user restores a deleted built-in Skill
- **THEN** the page SHALL call the frontend service boundary and refresh built-in Skill availability

### Requirement: Skill drift banner
The Skills settings page SHALL display a drift banner when Skill registry, source files, or mount paths are inconsistent.

#### Scenario: Display drift issues
- **WHEN** drift detection reports one or more issues
- **THEN** the page SHALL show a banner with the issue count and a path to review or synchronize the issues

#### Scenario: Synchronize drift
- **WHEN** a user activates one-click drift synchronization
- **THEN** the page SHALL call the frontend service boundary and display the synchronization report, including backup and overwrite results

### Requirement: Basic Settings log management section
The Basic Settings page SHALL provide a log management section for the active runtime.

#### Scenario: Display desktop log directory
- **WHEN** the Basic Settings page loads in the Tauri desktop runtime
- **THEN** it SHALL display the active log directory from the settings service

#### Scenario: Change desktop log directory
- **WHEN** a user changes the log directory from Basic Settings
- **THEN** the page SHALL save the directory through the settings service without calling Tauri APIs directly

#### Scenario: Open desktop log directory
- **WHEN** a user selects the open log directory action in the Tauri desktop runtime
- **THEN** the page SHALL request the action through the settings service

#### Scenario: Display logging policies
- **WHEN** the Basic Settings page displays log management
- **THEN** it SHALL show that retention is fixed at 30 days, archival is automatic, redaction is built in, and supported log levels are `error`, `warn`, `info`, and `debug`

#### Scenario: Disable native open action in Web runtime
- **WHEN** the Basic Settings page runs with the Web/mock adapter
- **THEN** it SHALL display the mock log path and keep the open log directory action disabled
