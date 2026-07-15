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
The system SHALL provide settings pages for basic configuration, provider management, SDK dependencies, MCP servers, agents, and skills.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include entries for basic configuration, provider management, SDK dependencies, MCP servers, agents, and skills

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
- **WHEN** Basic Configuration, Provider Management, SDK Dependencies, MCP Servers, Agents, or Skills content exceeds the visible settings content area
- **THEN** the active page SHALL scroll internally while the settings top navigation and left menu remain fixed in place
