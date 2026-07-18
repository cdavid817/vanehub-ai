# desktop-startup-controls Specification

## Purpose
TBD - created by archiving change optimize-settings-ui-and-runtime-controls. Update Purpose after archive.
## Requirements
### Requirement: Service-backed launch-on-startup control
The system SHALL provide a launch-on-startup setting through the shared settings service boundary.

#### Scenario: Display desktop startup setting
- **WHEN** the Basic Configuration page renders in the Tauri desktop runtime
- **THEN** it SHALL show the current launch-on-startup state from the settings service
- **AND** it SHALL NOT call a Tauri API directly from a React component

#### Scenario: Enable launch on startup
- **WHEN** the user enables launch-on-startup from Basic Configuration in the Tauri desktop runtime
- **THEN** the settings service SHALL persist the desired value and register VaneHub with the desktop startup mechanism

#### Scenario: Disable launch on startup
- **WHEN** the user disables launch-on-startup from Basic Configuration in the Tauri desktop runtime
- **THEN** the settings service SHALL persist the desired value and unregister VaneHub from the desktop startup mechanism

#### Scenario: Show startup unavailable in Web runtime
- **WHEN** Basic Configuration renders in the Web/mock runtime
- **THEN** it SHALL keep the launch-on-startup control unavailable with localized explanatory text
- **AND** the Web adapter SHALL remain interface-compatible with the desktop adapter

### Requirement: Official desktop autostart integration
The desktop runtime SHALL use the official Tauri autostart integration for startup registration instead of feature-local startup files or direct React-side platform logic.

#### Scenario: Synchronize startup registration through native runtime
- **WHEN** a launch-on-startup setting is saved in the Tauri desktop runtime
- **THEN** the native runtime SHALL synchronize the official autostart registration behind a Tauri command
- **AND** failures SHALL be returned to the frontend as user-displayable errors

#### Scenario: Preserve persisted desired state
- **WHEN** VaneHub starts after a launch-on-startup preference was saved
- **THEN** the settings service SHALL report the persisted desired state consistently with app settings
