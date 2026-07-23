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
The system SHALL provide primary settings navigation for basic configuration, CLI management, CLI parameter management, MCP servers, agents, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, and product information, while retaining SDK dependency management outside the primary navigation.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include primary entries for basic configuration, CLI management, CLI parameter management, MCP servers, agents, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, and about
- **AND** the CLI parameter management entry SHALL appear immediately after CLI management
- **AND** the SDK Dependencies page SHALL NOT appear as a primary settings navigation item
- **AND** Extension Capabilities SHALL appear below the higher-frequency agent, skill, and IM management entries
- **AND** the plugin integrations entry SHALL appear after Extension Capabilities
- **AND** the about entry SHALL be the final settings navigation item

#### Scenario: Display pages without backend services
- **WHEN** a user opens a settings page that does not yet have a dedicated frontend service boundary
- **THEN** the system SHALL render that page using frontend-local data without calling Tauri commands directly from React components

#### Scenario: Display About product information
- **WHEN** a user opens the About settings page in the Tauri desktop runtime or browser Web runtime
- **THEN** the page SHALL display localized product identity, build metadata, GitHub repository, changelog, update-check controls, and product positioning
- **AND** the page SHALL group product identity, software metadata, repository links, and update status in one software details panel
- **AND** the page SHALL group changelog and product positioning in one related information panel
- **AND** product details SHALL render without requiring a backend call
- **AND** the page SHALL NOT display removed runtime/agent or local CLI environment sections

#### Scenario: Check updates from About page
- **WHEN** a user activates the About page check-update action
- **THEN** the page SHALL check the latest GitHub release through a frontend service boundary
- **AND** the page SHALL show a localized checking, up-to-date, update-available, or failed state without blocking settings navigation

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
The system SHALL render the SDK dependencies page as a service-backed management surface rather than a static demo data page, while retaining it outside the primary settings navigation.

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

#### Scenario: Hide SDK from primary navigation
- **WHEN** the settings sidebar or settings page registry is used to render primary navigation
- **THEN** SDK Dependencies SHALL be omitted without deleting the SDK service or native implementation

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
The settings center SHALL remain reachable from the workspace activity bar and SHALL be the unified destination for the six tool shortcuts removed from the workspace session sidebar.

#### Scenario: Open settings from workspace activity entry
- **WHEN** the user activates the workspace Settings activity button
- **THEN** the system SHALL open the settings center without requiring a runtime-specific backend call

#### Scenario: Preserve settings page behavior
- **WHEN** the settings center is opened from the workspace activity bar
- **THEN** the settings center SHALL preserve existing navigation, page mounting, visual style, and service boundary behavior

### Requirement: Independent settings page scrolling
Each settings page SHALL scroll within its own content region without moving the settings top navigation or left menu.

#### Scenario: Scroll long settings page content
- **WHEN** Basic Configuration, CLI Management, SDK Dependencies, MCP Servers, Agents, or Skills content exceeds the visible settings content area
- **THEN** the active page SHALL scroll internally while the settings top navigation and left menu remain fixed in place

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

### Requirement: Polished settings visual system
The settings center SHALL apply the shared visual design system consistently across the shell, navigation, page headers, page sections, cards, forms, tables, filters, and operation panels.

#### Scenario: Settings shell visual consistency
- **WHEN** the settings center shell renders
- **THEN** top navigation, sidebar navigation, page content, and fixed scroll regions SHALL share consistent typography, spacing, border strength, panel treatment, hover states, and focus rings
- **AND** the visual result SHALL remain coherent in both `futuristic` and `minimal` styles

#### Scenario: Settings page visual consistency
- **WHEN** Basic Configuration, CLI Management, SDK Dependencies, MCP Servers, Agents, or Skills pages render
- **THEN** page headers, stat summaries, section panels, cards, form controls, empty states, status messages, and operation logs SHALL use shared primitives or shared visual classes
- **AND** page-specific styling SHALL not create a conflicting radius, color, or spacing system

### Requirement: Icon-enhanced settings interactions
The settings center SHALL use icons to improve scanability of navigation and high-frequency actions.

#### Scenario: Settings navigation icons
- **WHEN** the settings sidebar renders page navigation
- **THEN** each navigation entry SHALL include a stable icon that reflects the page purpose
- **AND** the active, hover, and disabled states SHALL remain legible in both registered styles

#### Scenario: Settings action icons
- **WHEN** a settings page renders refresh, install, update, rollback, delete, import, export, add, edit, filter, copy, open, or settings actions
- **THEN** the action SHALL include a consistent icon unless the control is purely textual by design
- **AND** icon-only actions SHALL expose a translated tooltip or accessible label

### Requirement: Settings theme refinement
The settings center SHALL visibly differentiate and polish both registered styles without changing page behavior.

#### Scenario: Futuristic style refinement
- **WHEN** `futuristic` style is active
- **THEN** settings surfaces SHALL use a dark operational appearance with subtle depth, restrained translucent or glass-like panels, clear blue primary accents, and readable muted text
- **AND** borders and shadows SHALL add structure without making the page look noisy

#### Scenario: Minimal style refinement
- **WHEN** `minimal` style is active
- **THEN** settings surfaces SHALL use a bright, crisp, low-shadow appearance with restrained borders, clear primary accents, and higher information density
- **AND** the style SHALL not rely on dark-only contrast assumptions from `futuristic`

### Requirement: Complete localized settings pages
All settings center pages and settings-owned dialogs SHALL render user-visible text through synchronized zh-CN and en translation resources.

#### Scenario: Agents settings page localized
- **WHEN** the Agents settings page renders in Simplified Chinese or English
- **THEN** its title, description, refresh action, filter controls, mode labels, configuration details, launch action, session detail labels, notices, and empty or error states SHALL use the active locale

#### Scenario: SDK settings page localized
- **WHEN** the SDK Dependencies page renders in Simplified Chinese or English
- **THEN** its title, description, refresh and update actions, stat cards, section headings, SDK status labels, version labels, operation actions, confirmations, notices, errors, empty states, and operation log labels SHALL use the active locale

#### Scenario: MCP settings page localized
- **WHEN** the MCP Servers page and its forms or import/export dialogs render in Simplified Chinese or English
- **THEN** titles, descriptions, actions, stat cards, scope labels, group labels, form labels, placeholders, validation messages, confirmations, notices, empty states, and modal controls SHALL use the active locale

#### Scenario: Existing settings translations corrected
- **WHEN** settings center locale resources contain equivalent zh-CN and en keys
- **THEN** each pair SHALL describe the same product concept and action semantics
- **AND** terminology for Agent, Skill, CLI, SDK, MCP, workspace, session, install, update, rollback, upgrade, and downgrade SHALL remain consistent across settings pages

### Requirement: Settings i18n regression coverage
The system SHALL include regression coverage that prevents settings pages from introducing untranslated visible text.

#### Scenario: Detect untranslated settings literals
- **WHEN** automated frontend tests run
- **THEN** they SHALL verify locale key parity
- **AND** they SHALL detect hard-coded user-visible strings in settings page components except for approved stable identifiers

### Requirement: Prompt Hooks settings navigation
The settings center SHALL include Prompt Hooks as a first-class settings page.

#### Scenario: Display Prompt Hooks navigation entry
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a localized Prompt Hooks entry with a stable icon
- **AND** the entry SHALL appear near Skills and CLI-related settings without making About cease to be the final settings navigation item

#### Scenario: Navigate to Prompt Hooks
- **WHEN** a user selects the Prompt Hooks navigation entry
- **THEN** the settings center SHALL render the Prompt Hooks settings page while preserving mounted state for other stateful settings pages

### Requirement: Rounded semantic settings icons
The settings center SHALL use consistent rounded icon containers and semantic icons for settings navigation and high-frequency settings actions.

#### Scenario: Render rounded navigation icons
- **WHEN** settings navigation renders in either registered visual style
- **THEN** page icons SHALL use stable dimensions, rounded geometry, semantic colors, and accessible labels without shifting layout on hover or active state

#### Scenario: Render desktop-control action icons
- **WHEN** Basic Configuration renders reset, open-directory, startup, data-management, log, proxy, or floating-assistant actions
- **THEN** actions SHALL use lucide or existing project icons where icons improve recognition

### Requirement: SSH connection settings navigation
The settings center SHALL include SSH connection management as a first-class settings page.

#### Scenario: Display SSH connection navigation entry
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a localized SSH connection management entry with a stable icon
- **AND** the About entry SHALL remain the final settings navigation item

#### Scenario: Navigate to SSH connection settings
- **WHEN** a user selects the SSH connection management entry
- **THEN** the settings center SHALL render the SSH connection settings page while preserving mounted state for other stateful settings pages

### Requirement: Lazy settings module loading
The settings center SHALL load designated heavy settings page modules on first visit while preserving the established mounted state of every visited page.

#### Scenario: Open settings before visiting a heavy page
- **WHEN** the settings center opens and a designated heavy page has not been visited
- **THEN** that page module SHALL remain unloaded
- **AND** the active settings page SHALL remain usable

#### Scenario: Visit a heavy settings page
- **WHEN** the user selects a designated heavy settings page for the first time
- **THEN** the settings content region SHALL show a localized loading state while its module loads
- **AND** the navigation and settings shell SHALL remain mounted

#### Scenario: Return to a visited lazy page
- **WHEN** the user leaves and later returns to a lazy-loaded settings page
- **THEN** its component SHALL remain mounted between visits
- **AND** its local form, filter, and scroll state SHALL be preserved

#### Scenario: Fail to load a settings module
- **WHEN** a lazy settings page module cannot be loaded
- **THEN** only that page content region SHALL show a localized retryable error
- **AND** the user SHALL be able to navigate to another settings page
