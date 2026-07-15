## ADDED Requirements

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
