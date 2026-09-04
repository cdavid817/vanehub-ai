## MODIFIED Requirements

### Requirement: UCD settings pages
The system SHALL provide primary settings navigation for basic configuration, CLI management, CLI parameter management, MCP servers, Agent configuration, expert roles, Personalization, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, product documentation, and product information, while retaining SDK dependency management outside the primary navigation and removing Agent Management without a replacement management destination.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include primary entries for basic configuration, CLI management, CLI parameter management, MCP servers, Agent configuration, expert roles, Personalization, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, documentation, and about
- **AND** the system SHALL NOT include a standalone Agent Management entry
- **AND** Agent Configuration SHALL NOT display registered-Agent inventory, registration, lifecycle, or runtime controls
- **AND** the expert roles entry SHALL appear after Agent configuration and before skills, because roles are assigned to Agents and referenced by Skills
- **AND** Expert Roles SHALL define reusable role identity and instructions only, and SHALL NOT become a replacement Agent management destination
- **AND** the CLI parameter management entry SHALL appear immediately after CLI management
- **AND** the SDK Dependencies page SHALL NOT appear as a primary settings navigation item
- **AND** Personalization SHALL appear after Agent Configuration and before Skills
- **AND** Extension Capabilities SHALL appear below the higher-frequency Agent configuration, skill, and IM management entries
- **AND** the plugin integrations entry SHALL appear after Extension Capabilities
- **AND** the documentation entry SHALL appear immediately before the about entry
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

## ADDED Requirements

### Requirement: Product documentation settings page
The settings center SHALL provide a documentation page that renders the product README shipped with the build, and SHALL make it the destination of the workspace Help entry.

#### Scenario: Open the documentation page
- **WHEN** a user opens the documentation settings page in the Tauri desktop runtime or browser Web runtime
- **THEN** the page SHALL render the bundled README as formatted document content
- **AND** it SHALL render without a network request and without calling a Tauri command from a React component

#### Scenario: Select documentation language
- **WHEN** the active application language's base language tag has a bundled README translation
- **THEN** the page SHALL render that translation, so a regional variant resolves to its base language's README
- **AND** when no bundled README matches that base language it SHALL render the English README

#### Scenario: Follow a documentation link
- **WHEN** a user activates a link inside the rendered documentation
- **THEN** the link SHALL open outside the document surface rather than replacing the settings center
- **AND** an image that cannot be resolved SHALL degrade without breaking the rest of the page

### Requirement: Settings navigation entry legibility
A settings navigation entry SHALL remain fully visible within the settings sidebar at every supported viewport width, including its selected-state highlight.

#### Scenario: Long navigation label
- **WHEN** a navigation entry's localized label is wider than the available sidebar width
- **THEN** the entry SHALL keep its full width inside the sidebar and truncate the label
- **AND** the entry's selected-state highlight SHALL NOT be clipped on its leading or trailing edge
- **AND** the full label SHALL remain available to pointer hover and to assistive technology

#### Scenario: Narrow layout navigation
- **WHEN** the settings sidebar collapses to its horizontal narrow-layout presentation
- **THEN** every navigation entry SHALL remain reachable by scrolling
- **AND** the selected entry SHALL remain fully visible when it is scrolled into view
