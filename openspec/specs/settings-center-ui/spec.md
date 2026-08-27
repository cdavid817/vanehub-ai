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
The system SHALL provide primary settings navigation for basic configuration, CLI management, CLI parameter management, MCP servers, Agent configuration, expert roles, Personalization, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, and product information, while retaining SDK dependency management outside the primary navigation and removing Agent Management without a replacement management destination.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include primary entries for basic configuration, CLI management, CLI parameter management, MCP servers, Agent configuration, expert roles, Personalization, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, and about
- **AND** the system SHALL NOT include a standalone Agent Management entry
- **AND** Agent Configuration SHALL NOT display registered-Agent inventory, registration, lifecycle, or runtime controls
- **AND** the expert roles entry SHALL appear after Agent configuration and before skills, because roles are assigned to Agents and referenced by Skills
- **AND** Expert Roles SHALL define reusable role identity and instructions only, and SHALL NOT become a replacement Agent management destination
- **AND** the CLI parameter management entry SHALL appear immediately after CLI management
- **AND** the SDK Dependencies page SHALL NOT appear as a primary settings navigation item
- **AND** Personalization SHALL appear after Agent Configuration and before Skills
- **AND** Extension Capabilities SHALL appear below the higher-frequency Agent configuration, skill, and IM management entries
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
The settings center SHALL load every service-backed settings page module on first visit while preserving the established mounted state of every visited page.

#### Scenario: Open settings before visiting another page
- **WHEN** the settings center opens and a settings page has not been visited
- **THEN** that page module SHALL remain unloaded and unmounted
- **AND** it SHALL NOT start service-backed page work
- **AND** the active settings page SHALL remain usable

#### Scenario: Visit a settings page
- **WHEN** the user selects a settings page for the first time
- **THEN** the settings content region SHALL show a localized loading state while its module loads
- **AND** the navigation and settings shell SHALL remain mounted

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

### Requirement: Personalization settings navigation
The settings center SHALL include Personalization as a first-class settings page, hosting host-level custom instructions and memory preferences and management, independent of the per-Agent configuration tabs.

#### Scenario: Display Personalization navigation entry
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a localized Personalization entry with a stable icon
- **AND** the About entry SHALL remain the final settings navigation item

#### Scenario: Navigate to Personalization settings
- **WHEN** a user selects the Personalization navigation entry
- **THEN** the settings center SHALL render the Personalization settings page while preserving mounted state for other stateful settings pages

### Requirement: Settings expose LSP configuration and runtime status
The Agent configuration area SHALL provide a localized service-backed LSP section with the master switch, one switch per registered language obtained from the service boundary, automatic discovery state, override controls whose meaning follows each language's backend-reported launch shape, bounded startup-argument controls, bounded initialization-options validation, trusted-workspace management, isolated server testing, and running-server status. The section SHALL render its language controls from the backend-supplied registered-language set, and its negotiated-capability rows from the backend-supplied negotiated method list, rather than from fixed lists compiled into the frontend. React components SHALL use the shared frontend service boundary, and desktop and Web adapters SHALL implement the same contract shape.

#### Scenario: User configures Rust LSP
- **WHEN** a user enables LSP and Rust, selects a discovered `rust-analyzer` or valid executable override, supplies valid bounded initialization options, and saves
- **THEN** the settings page SHALL submit the normalized configuration through the service boundary
- **AND** it SHALL refresh discovery and affected server status without calling Tauri directly

#### Scenario: Initialization options are invalid
- **WHEN** a user attempts to save malformed, non-object, or oversized initialization-options JSON
- **THEN** shared form validation SHALL reject the submission
- **AND** the last valid persisted configuration SHALL remain active

#### Scenario: User trusts a workspace
- **WHEN** a user grants LSP trust to a canonical local workspace
- **THEN** the UI SHALL explain that a language server is a local executable with the user's operating-system permissions
- **AND** the trusted-workspace list SHALL refresh through the service boundary

#### Scenario: Runtime status is displayed
- **WHEN** one or more language-server instances are starting, ready, backing off, stopping, or failed
- **THEN** the status surface SHALL show safe server identity, language, relative project root, lifecycle state, restart count, last response, and diagnostic count when available
- **AND** it SHALL NOT claim portable memory or indexed-file metrics that the server does not provide

#### Scenario: Web runtime opens LSP settings
- **WHEN** the LSP settings section is used in browser Web mode
- **THEN** it SHALL support deterministic mock configuration, trust, discovery, testing, and status behavior
- **AND** it SHALL not require a native filesystem or process

#### Scenario: Registered language set determines the rendered controls
- **WHEN** the settings section loads the registered-language set through the service boundary
- **THEN** it SHALL render exactly one language control group per registered language, each with that language's own discovery state, override, startup arguments, and initialization options
- **AND** adding a language to the backend registry SHALL require no new per-language frontend component

#### Scenario: A language's override names a directory rather than a file
- **WHEN** the backend reports a language whose launch shape takes an install directory
- **THEN** the override control SHALL describe and validate a directory rather than an executable file
- **AND** it SHALL do so from the reported launch shape, not from the language's identity, so a second such language needs no frontend change

#### Scenario: A prerequisite runtime is missing
- **WHEN** discovery reports that a language's prerequisite runtime is absent
- **THEN** the language card SHALL present that as its own state, distinct from an unset install directory and from a directory missing its launcher
- **AND** it SHALL name the runtime the user has to install rather than reporting a generic unavailable server

#### Scenario: Language is unsupported on this host
- **WHEN** a registered language declares no applicability for the current operating system
- **THEN** its control group SHALL present it as unsupported on this host and SHALL NOT offer enablement or server testing
- **AND** it SHALL be distinguishable from a supported language whose executable was simply not discovered

#### Scenario: Startup arguments are invalid
- **WHEN** a user attempts to save startup arguments that are not a bounded list of strings or that exceed the declared size limit
- **THEN** shared form validation SHALL reject the submission
- **AND** the last valid persisted configuration SHALL remain active

#### Scenario: Negotiated method list determines the rendered capability rows
- **WHEN** the status surface renders a ready server's negotiated capabilities
- **THEN** it SHALL render one supported-or-unsupported row per method the backend reports, in the order reported
- **AND** adding a method to the backend SHALL require no new frontend row

#### Scenario: A reported method has no localized label
- **WHEN** the backend reports a negotiated method whose localization key is absent from the active locale
- **THEN** the row SHALL fall back to the raw method identifier
- **AND** it SHALL NOT render the missing key or an empty label

### Requirement: Workflow-oriented settings navigation order
The Settings sidebar SHALL order destinations by expected workflow frequency: general setup and recurring Agent behavior first, reusable capabilities and customization next, one-time CLI installation and external integrations after that, and diagnostics and product information last.

#### Scenario: Render settings destinations
- **WHEN** the Settings sidebar renders
- **THEN** destinations SHALL appear in the order Basic, Agent Configuration, Agent Policies, CLI Parameters, MCP, Skills, Personalization, Prompt Hooks, Expert Roles, CLI Management, Extensions, Plugin Integrations, IM, SSH Connections, Observability, Usage Statistics, and About
- **AND** existing destination ids and deep-link behavior SHALL remain unchanged

### Requirement: CLI Management operational summary and filters

The CLI Management page SHALL present a compact operational summary and filters derived from normalized backend snapshots.

#### Scenario: Display summary

- **WHEN** CLI snapshots are available
- **THEN** the page SHALL show counts for ready, needs login, update available, conflicts, and broken
- **AND** it SHALL avoid oversized dashboard cards that reduce desktop information density

#### Scenario: Filter CLI tools

- **WHEN** the user searches or filters by status, source, or needs-action state
- **THEN** the visible tool list SHALL update without changing backend lifecycle policy
- **AND** filter and scroll state SHALL remain mounted across Settings navigation

### Requirement: Orthogonal CLI status presentation

The CLI Management page SHALL distinguish health, authentication, compatibility, update state, source, freshness, and manageability.

#### Scenario: Healthy detect-only installation

- **WHEN** a tool is healthy but its source is detect-only
- **THEN** the page SHALL show healthy executable status and detect-only management separately
- **AND** it SHALL not use a broken/error presentation merely because VaneHub cannot mutate it

#### Scenario: Tool needs authentication

- **WHEN** authentication is required
- **THEN** the page SHALL show needs-login/readiness state independently from installation and executable health

#### Scenario: Data is stale

- **WHEN** a snapshot is stale or refreshing
- **THEN** the page SHALL keep the last known values visible with freshness and last-checked information

### Requirement: Backend-authoritative CLI actions

The CLI Management page SHALL render only backend-derived allowed actions and SHALL not compare versions or infer lifecycle support in React.

#### Scenario: Primary action is rendered

- **WHEN** the backend returns one or more allowed actions
- **THEN** the page SHALL show one contextually appropriate primary action and place remaining actions in an accessible menu

#### Scenario: Current version is selected

- **WHEN** the selected target equals the active version
- **THEN** the page SHALL show current state
- **AND** no install, upgrade, or downgrade action SHALL be enabled

#### Scenario: No automatic action is allowed

- **WHEN** the backend returns no mutation action
- **THEN** the page SHALL show the backend reason and safe guidance rather than constructing a fallback action

### Requirement: CLI environment details drawer

The CLI Management page SHALL provide an accessible details drawer with Overview, Installations, Diagnostics, and Operations sections.

#### Scenario: View overview

- **WHEN** a user opens a tool's details
- **THEN** the drawer SHALL show overall and orthogonal states, active path/version/source, freshness, update source/channel, and last mutation outcome

#### Scenario: View installations

- **WHEN** the Installations section is selected
- **THEN** the drawer SHALL show bounded discovered paths, versions, sources, confidence, PATH priority, executable state, and active/shadowed identity

#### Scenario: View diagnostics

- **WHEN** the Diagnostics section is selected
- **THEN** the drawer SHALL show normalized version, Doctor, authentication, dependency, and compatibility results
- **AND** it SHALL provide a service-backed rerun action

#### Scenario: View operations

- **WHEN** the Operations section is selected
- **THEN** the drawer SHALL show related queued/running/terminal operations, phases, outcomes, timestamps, and bounded redacted logs

### Requirement: CLI action-plan review dialog

The UI SHALL require review of a persisted action plan before every CLI machine mutation.

#### Scenario: Review plan

- **WHEN** planning succeeds
- **THEN** the dialog SHALL show action, source, current and target versions, channel, structured command preview, network access, elevation, preconditions, warnings, expiry, and the absence of automatic source fallback

#### Scenario: Confirm plan

- **WHEN** the user confirms a non-stale plan
- **THEN** the page SHALL execute the plan id and revision through the service
- **AND** it SHALL not submit a command string or recompute the action

#### Scenario: Plan expires or becomes stale

- **WHEN** the backend rejects the plan as expired or stale
- **THEN** the dialog SHALL close or disable confirmation, explain that the environment changed, and offer preparation of a new plan

### Requirement: CLI bulk upgrade preview

The CLI Management page SHALL show a persisted bulk upgrade preview before starting any bulk mutation.

#### Scenario: Preview eligible items

- **WHEN** a bulk plan is prepared
- **THEN** the dialog SHALL list each eligible tool, source, current-to-target transition, elevation/network requirement, and warning

#### Scenario: Preview skipped items

- **WHEN** one or more tools are already current, detect-only, broken, unauthenticated, catalog-unavailable, or otherwise ineligible
- **THEN** the dialog SHALL list each skipped tool and localized reason

#### Scenario: Observe item outcomes

- **WHEN** bulk execution runs
- **THEN** the page SHALL show queued/running/terminal state and final outcome for every item
- **AND** one stale or failed item SHALL not erase other item results

### Requirement: CLI operation interaction

The CLI Management page SHALL display per-tool operation state, bounded logs, cancellation where supported, and partial-completion guidance.

#### Scenario: One tool operation runs

- **WHEN** one CLI operation is queued or running
- **THEN** only the affected tool SHALL show its active state
- **AND** unrelated tools SHALL remain inspectable

#### Scenario: Operation is cancellable

- **WHEN** the operation contract reports `cancellable`
- **THEN** the UI SHALL expose a cancel action through `OperationService`

#### Scenario: Mutation is applied but unverified

- **WHEN** the terminal result is applied-unverified
- **THEN** the UI SHALL state that the package command completed but verification failed
- **AND** it SHALL offer refresh or diagnostics without claiming rollback

#### Scenario: Command failed but machine changed

- **WHEN** the terminal result is changed-but-failed
- **THEN** the UI SHALL state that the machine appears to have changed despite the failed or cancelled command
- **AND** it SHALL display the observed snapshot and diagnostic action

### Requirement: Accessible and localized CLI Management

Every CLI Management control, state, dialog, drawer, tooltip, empty state, and error SHALL be accessible and localized in every registered application locale.

#### Scenario: Use keyboard and assistive technology

- **WHEN** a user navigates cards, menus, expanders, drawer tabs, dialogs, logs, or cancel actions by keyboard or assistive technology
- **THEN** focus order, focus restoration, labels, `aria-expanded`, `aria-controls`, dialog semantics, and non-color status cues SHALL be correct
- **AND** operation announcements SHALL not repeatedly announce streaming log lines

#### Scenario: Render registered locale

- **WHEN** the active application locale changes
- **THEN** all user-owned CLI Management labels and messages SHALL use matching locale resources
- **AND** dates SHALL use the active locale

#### Scenario: Render both visual styles

- **WHEN** `futuristic` or `minimal` style is active at desktop or narrow width
- **THEN** the page SHALL use shared semantic tokens, compact density, stable control dimensions, and no nested card-in-card layout

