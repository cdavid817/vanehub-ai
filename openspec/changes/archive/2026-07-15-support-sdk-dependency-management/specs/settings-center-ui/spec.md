## ADDED Requirements

### Requirement: Service-backed SDK settings page
The system SHALL render the SDK dependencies page as a service-backed management surface rather than a static demo data page.

#### Scenario: Display SDK dependency statuses
- **WHEN** a user opens the SDK dependencies settings page
- **THEN** the page SHALL load managed SDK dependency statuses through the SDK frontend service interface

#### Scenario: Manage SDK dependencies from settings
- **WHEN** a user refreshes, checks versions, installs, updates, rolls back, or uninstalls an SDK dependency from the settings page
- **THEN** the page SHALL perform those operations through the SDK frontend service interface

#### Scenario: Display SDK operation logs
- **WHEN** an SDK install, update, rollback, or uninstall operation produces logs
- **THEN** the SDK settings page SHALL display those logs in the page while preserving the selected SDK page state

#### Scenario: Preserve settings page style
- **WHEN** the SDK dependencies page renders service-backed data and controls
- **THEN** the page SHALL use the shared settings center layout, semantic design tokens, controls, and status styles consistently with the rest of the settings center

### Requirement: SDK version action controls
The system SHALL present selectable SDK versions and derive the primary action from installed state and selected target version.

#### Scenario: Install action for missing SDK
- **WHEN** an SDK is not installed and a target version is selected
- **THEN** the page SHALL present an install action for that target version

#### Scenario: Update action for newer version
- **WHEN** an SDK is installed and the selected target version is newer than the installed version
- **THEN** the page SHALL present an update action for that target version

#### Scenario: Rollback action for older version
- **WHEN** an SDK is installed and the selected target version is older than the installed version
- **THEN** the page SHALL present a rollback action for that target version

#### Scenario: Current version action disabled
- **WHEN** an SDK is installed and the selected target version equals the installed version
- **THEN** the page SHALL present the current-version state and prevent a redundant install operation
