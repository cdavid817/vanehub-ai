## ADDED Requirements

### Requirement: Native unified logging service
The native runtime SHALL provide a unified logging service for diagnostics and operation logs.

#### Scenario: Write diagnostic log entry
- **WHEN** a native command, storage operation, validation path, network operation, or task fails or emits diagnostics
- **THEN** the native runtime SHALL write a redacted structured log entry through the unified logging service

#### Scenario: Write operation log entry
- **WHEN** a backend-managed operation emits progress, stdout, stderr, completion, or failure output
- **THEN** the native runtime SHALL write a redacted operation log entry through the unified logging service

#### Scenario: Use configured log directory
- **WHEN** the logging service writes a log entry
- **THEN** it SHALL write under the currently configured log directory

### Requirement: Native log directory command
The native runtime SHALL expose declared Tauri commands for log directory metadata, log directory changes, and opening the active log directory.

#### Scenario: Open log directory command
- **WHEN** the frontend settings service requests to open the active log directory
- **THEN** the native runtime SHALL open the directory without exposing unrestricted filesystem APIs to React components

#### Scenario: Save log directory command
- **WHEN** the frontend settings service saves a log directory
- **THEN** the native runtime SHALL validate or create the directory before persisting the setting
