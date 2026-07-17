## MODIFIED Requirements

### Requirement: Unified tool entry from workspace
The settings center SHALL remain reachable from the workspace activity bar and SHALL be the unified destination for the six tool shortcuts removed from the workspace session sidebar.

#### Scenario: Open settings from workspace activity entry
- **WHEN** the user activates the workspace Settings activity button
- **THEN** the system SHALL open the settings center without requiring a runtime-specific backend call

#### Scenario: Preserve settings page behavior
- **WHEN** the settings center is opened from the workspace activity bar
- **THEN** the settings center SHALL preserve existing navigation, page mounting, visual style, and service boundary behavior
