## ADDED Requirements

### Requirement: Prompt Hooks settings navigation
The settings center SHALL include Prompt Hooks as a first-class settings page.

#### Scenario: Display Prompt Hooks navigation entry
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a localized Prompt Hooks entry with a stable icon
- **AND** the entry SHALL appear near Skills and CLI-related settings without making About cease to be the final settings navigation item

#### Scenario: Navigate to Prompt Hooks
- **WHEN** a user selects the Prompt Hooks navigation entry
- **THEN** the settings center SHALL render the Prompt Hooks settings page while preserving mounted state for other stateful settings pages
