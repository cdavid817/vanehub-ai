## ADDED Requirements

### Requirement: SSH connection settings navigation
The settings center SHALL include SSH connection management as a first-class settings page.

#### Scenario: Display SSH connection navigation entry
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a localized SSH connection management entry with a stable icon
- **AND** the About entry SHALL remain the final settings navigation item

#### Scenario: Navigate to SSH connection settings
- **WHEN** a user selects the SSH connection management entry
- **THEN** the settings center SHALL render the SSH connection settings page while preserving mounted state for other stateful settings pages
