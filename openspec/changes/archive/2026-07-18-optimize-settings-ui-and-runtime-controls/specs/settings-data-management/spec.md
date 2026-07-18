## ADDED Requirements

### Requirement: Basic Settings data management section
The Basic Configuration page SHALL provide a Data Management section for local application data owned by the active runtime.

#### Scenario: Display desktop database location
- **WHEN** Basic Configuration loads in the Tauri desktop runtime
- **THEN** it SHALL display the local SQLite database path or containing directory through the settings service

#### Scenario: Open database directory
- **WHEN** the user selects the open database directory action in the Tauri desktop runtime
- **THEN** the page SHALL request the action through the settings service
- **AND** the native runtime SHALL open the directory containing the SQLite database instead of opening the database file directly

#### Scenario: Keep SQLite out of React
- **WHEN** the Basic Configuration page renders the Data Management section
- **THEN** React components SHALL NOT access SQLite, derive native app data paths, or call Tauri commands directly

#### Scenario: Display Web runtime limitation
- **WHEN** Basic Configuration runs through the Web/mock adapter
- **THEN** the Data Management section SHALL remain visible or gracefully unavailable with localized text
- **AND** local database opening SHALL be disabled or rejected through the Web settings adapter
