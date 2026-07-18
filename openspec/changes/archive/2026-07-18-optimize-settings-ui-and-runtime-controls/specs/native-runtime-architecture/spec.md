## ADDED Requirements

### Requirement: Native settings commands for local data and startup
The native runtime SHALL expose settings-adapter commands for opening the SQLite database directory and managing launch-on-startup registration.

#### Scenario: Open database directory from native command
- **WHEN** the Tauri settings adapter requests opening the database directory
- **THEN** the native runtime SHALL resolve the active SQLite database path from the registry store and open its containing directory
- **AND** it SHALL NOT expose direct SQLite access to React components

#### Scenario: Return database location information
- **WHEN** the Tauri settings adapter requests settings or data-management metadata
- **THEN** the native runtime SHALL provide user-displayable database location information without requiring the frontend to infer app data paths

#### Scenario: Manage startup registration from native command
- **WHEN** the Tauri settings adapter saves launch-on-startup
- **THEN** the native runtime SHALL synchronize the official Tauri autostart registration and return success or a sanitized user-displayable failure

#### Scenario: Preserve command boundary errors
- **WHEN** database-directory opening or startup registration fails across the Tauri command boundary
- **THEN** the command SHALL convert the error to `Result<T, String>` or the project's command-safe error shape
