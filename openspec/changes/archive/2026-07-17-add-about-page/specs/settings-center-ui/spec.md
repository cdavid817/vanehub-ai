## MODIFIED Requirements

### Requirement: UCD settings pages
The system SHALL provide settings pages for basic configuration, CLI management, SDK dependencies, MCP servers, agents, skills, and product information.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include entries for basic configuration, CLI management, SDK dependencies, MCP servers, agents, skills, and about
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
