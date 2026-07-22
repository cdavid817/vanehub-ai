## MODIFIED Requirements

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
