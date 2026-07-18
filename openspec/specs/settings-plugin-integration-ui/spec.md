# settings-plugin-integration-ui Specification

## Purpose
TBD - created by archiving change add-github-plugin-integration. Update Purpose after archive.
## Requirements
### Requirement: Plugin Integrations settings page
The settings center SHALL provide a Plugin Integrations page for managing built-in product integrations, including the built-in GitHub integration.

#### Scenario: Navigate to Plugin Integrations
- **WHEN** the settings sidebar renders
- **THEN** it SHALL include a localized Plugin Integrations entry after Extension Capabilities and before MCP Servers

#### Scenario: Display GitHub plugin card
- **WHEN** the Plugin Integrations page loads
- **THEN** it SHALL show the built-in GitHub plugin card from the plugin integration service rather than hard-coded page-only data

### Requirement: GitHub plugin setup and readiness UI
The Plugin Integrations page SHALL show GitHub setup guidance, configuration status, and a connection test action.

#### Scenario: Show setup guidance
- **WHEN** the GitHub plugin card renders
- **THEN** it SHALL show localized setup steps for GitHub CLI authentication and a safe official documentation link

#### Scenario: Test GitHub readiness
- **WHEN** the user activates the GitHub test action
- **THEN** the page SHALL call the plugin integration service, show loading or running feedback, and display the terminal configured, missing CLI, unauthenticated, unavailable, or error status

#### Scenario: Desktop-only Web limitation
- **WHEN** the page runs through the Web/mock adapter
- **THEN** it SHALL display a localized limitation that live GitHub readiness testing requires the desktop runtime

### Requirement: Plugin integration search and summaries
The Plugin Integrations page SHALL support settings search and summary status cards for built-in integrations.

#### Scenario: Search plugin integrations
- **WHEN** the user enters a settings search term on the Plugin Integrations page
- **THEN** the visible plugin cards SHALL be filtered by localized name, description, setup text, status text, and stable id

#### Scenario: Display plugin summary
- **WHEN** plugin integration data is loaded
- **THEN** the page SHALL show localized summary counts for total integrations, configured integrations, and integrations requiring attention

### Requirement: Plugin integration visual-style consistency
The Plugin Integrations page SHALL use shared settings layout components and semantic design tokens without branching on theme names.

#### Scenario: Render both registered styles
- **WHEN** either `futuristic` or `minimal` style is active
- **THEN** GitHub plugin cards, status badges, setup steps, links, buttons, empty states, loading states, and errors SHALL remain readable and visually consistent with the rest of the settings center

### Requirement: Localized plugin integration text
All Plugin Integrations user-visible text SHALL use synchronized Simplified Chinese and English translation resources.

#### Scenario: Switch application language
- **WHEN** the active application language changes between `zh-CN` and `en`
- **THEN** navigation, headings, descriptions, setup steps, statuses, actions, notices, errors, and search placeholders on the Plugin Integrations page SHALL render in the active locale

#### Scenario: Maintain translation parity
- **WHEN** plugin integration translation keys are added or changed
- **THEN** the existing i18n resource parity check SHALL require matching keys in both locale files

