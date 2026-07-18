## MODIFIED Requirements

### Requirement: UCD settings pages
The system SHALL provide primary settings navigation for basic configuration, CLI management, CLI parameter management, MCP servers, agents, skills, IM connectors, extension capabilities, usage statistics, and product information, while retaining SDK dependency management outside the primary navigation.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include primary entries for basic configuration, CLI management, CLI parameter management, MCP servers, agents, skills, IM connectors, extension capabilities, usage statistics, and about
- **AND** the CLI parameter management entry SHALL appear immediately after CLI management
- **AND** the SDK Dependencies page SHALL NOT appear as a primary settings navigation item
- **AND** Extension Capabilities SHALL appear below the higher-frequency agent, skill, and IM management entries
- **AND** the about entry SHALL be the final settings navigation item

#### Scenario: Display pages without backend services
- **WHEN** a user opens a settings page that does not yet have a dedicated frontend service boundary
- **THEN** the system SHALL render that page using frontend-local data without calling Tauri commands directly from React components

#### Scenario: Display About product information
- **WHEN** a user opens the About settings page in the Tauri desktop runtime or browser Web runtime
- **THEN** the page SHALL display localized product identity, supported runtimes, supported AI coding agents, GitHub repository, changelog, update-check controls, and build metadata
- **AND** product details SHALL render without requiring a backend call

#### Scenario: Check updates from About page
- **WHEN** a user activates the About page check-update action
- **THEN** the page SHALL check the latest GitHub release through a frontend service boundary
- **AND** the page SHALL show a localized checking, up-to-date, update-available, or failed state without blocking settings navigation

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

## ADDED Requirements

### Requirement: Rounded semantic settings icons
The settings center SHALL use consistent rounded icon containers and semantic icons for settings navigation and high-frequency settings actions.

#### Scenario: Render rounded navigation icons
- **WHEN** settings navigation renders in either registered visual style
- **THEN** page icons SHALL use stable dimensions, rounded geometry, semantic colors, and accessible labels without shifting layout on hover or active state

#### Scenario: Render desktop-control action icons
- **WHEN** Basic Configuration renders reset, open-directory, startup, data-management, log, proxy, or floating-assistant actions
- **THEN** actions SHALL use lucide or existing project icons where icons improve recognition
