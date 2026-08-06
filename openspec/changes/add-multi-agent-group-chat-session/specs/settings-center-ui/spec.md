## MODIFIED Requirements

### Requirement: UCD settings pages
The system SHALL provide primary settings navigation for basic configuration, CLI management, CLI parameter management, MCP servers, Agent configuration, expert roles, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, and product information, while retaining SDK dependency management outside the primary navigation and removing Agent Management without a replacement management destination.

#### Scenario: Display UCD page set
- **WHEN** the settings center navigation is rendered
- **THEN** the system SHALL include primary entries for basic configuration, CLI management, CLI parameter management, MCP servers, Agent configuration, expert roles, skills, Prompt Hooks, IM connectors, extension capabilities, plugin integrations, usage statistics, and about
- **AND** the system SHALL NOT include a standalone Agent Management entry
- **AND** Agent Configuration SHALL NOT display registered-Agent inventory, registration, lifecycle, or runtime controls
- **AND** the expert roles entry SHALL appear after Agent configuration and before skills, because roles are assigned to Agents and referenced by Skills
- **AND** Expert Roles SHALL define reusable role identity and instructions only, and SHALL NOT become a replacement Agent management destination
- **AND** the CLI parameter management entry SHALL appear immediately after CLI management
- **AND** the SDK Dependencies page SHALL NOT appear as a primary settings navigation item
- **AND** Extension Capabilities SHALL appear below the higher-frequency Agent configuration, skill, and IM management entries
- **AND** the plugin integrations entry SHALL appear after Extension Capabilities
- **AND** the about entry SHALL be the final settings navigation item
