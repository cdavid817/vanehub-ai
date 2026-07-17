## MODIFIED Requirements

### Requirement: UCD settings pages
The system SHALL provide settings pages for basic configuration, CLI management, CLI parameter management, SDK dependencies, MCP servers, agents, skills, and product information.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include entries for basic configuration, CLI management, CLI parameter management, SDK dependencies, MCP servers, agents, skills, and about
- **AND** the CLI parameter management entry SHALL appear immediately after CLI management
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

## ADDED Requirements

### Requirement: Service-backed CLI parameter settings page
The settings center SHALL render CLI Parameter Management as a service-backed page separate from CLI installation and version management.

#### Scenario: Open CLI parameter page
- **WHEN** a user opens CLI Parameter Management
- **THEN** the page SHALL load typed profiles through the frontend agent service
- **AND** it SHALL preserve the settings shell, independent content scrolling, search behavior, and mounted draft state

#### Scenario: Keep installation management separate
- **WHEN** the CLI parameter page renders
- **THEN** it SHALL NOT install, upgrade, downgrade, detect, or remove a CLI package
- **AND** CLI package operations SHALL remain on the existing CLI Management page

