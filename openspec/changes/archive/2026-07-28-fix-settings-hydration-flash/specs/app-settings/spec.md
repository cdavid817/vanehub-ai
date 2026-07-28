## MODIFIED Requirements

### Requirement: Settings persistence
The system SHALL persist common settings through the active runtime adapter and SHALL complete initial settings hydration before displaying the formal application surface.

#### Scenario: Persist desktop setting
- **WHEN** the application runs in the Tauri desktop runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through a Tauri command backed by SQLite storage

#### Scenario: Persist Web setting
- **WHEN** the application runs in the browser Web runtime and a user saves a common setting
- **THEN** the system SHALL persist the setting through the Web adapter without requiring a Tauri command

#### Scenario: Restore saved settings
- **WHEN** the application starts after common settings have been saved
- **THEN** the system SHALL restore and apply the saved setting values for the active runtime
- **AND** the formal application surface SHALL first become visible with the restored root font size, visual theme, and application language already applied

#### Scenario: Fall back when initial settings cannot be loaded
- **WHEN** the active runtime fails to load common settings during application startup
- **THEN** the system SHALL apply the shared default settings before displaying the formal application surface
- **AND** the settings provider SHALL retain a user-displayable error without preventing startup
