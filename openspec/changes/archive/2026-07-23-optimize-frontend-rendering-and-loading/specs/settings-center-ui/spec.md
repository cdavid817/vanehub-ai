## ADDED Requirements

### Requirement: Lazy settings module loading
The settings center SHALL load designated heavy settings page modules on first visit while preserving the established mounted state of every visited page.

#### Scenario: Open settings before visiting a heavy page
- **WHEN** the settings center opens and a designated heavy page has not been visited
- **THEN** that page module SHALL remain unloaded
- **AND** the active settings page SHALL remain usable

#### Scenario: Visit a heavy settings page
- **WHEN** the user selects a designated heavy settings page for the first time
- **THEN** the settings content region SHALL show a localized loading state while its module loads
- **AND** the navigation and settings shell SHALL remain mounted

#### Scenario: Return to a visited lazy page
- **WHEN** the user leaves and later returns to a lazy-loaded settings page
- **THEN** its component SHALL remain mounted between visits
- **AND** its local form, filter, and scroll state SHALL be preserved

#### Scenario: Fail to load a settings module
- **WHEN** a lazy settings page module cannot be loaded
- **THEN** only that page content region SHALL show a localized retryable error
- **AND** the user SHALL be able to navigate to another settings page
