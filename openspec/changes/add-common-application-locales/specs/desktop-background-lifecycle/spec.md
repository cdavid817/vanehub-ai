## MODIFIED Requirements

### Requirement: Desktop close-to-tray behavior
The Tauri desktop runtime SHALL keep the VaneHub process and enabled IM connectors running when the user closes the main window.

#### Scenario: Close main window
- **WHEN** the user requests to close the main desktop window without explicitly quitting VaneHub
- **THEN** the runtime SHALL prevent process exit, hide the window, and keep enabled connectors running

#### Scenario: Explain first close-to-tray action
- **WHEN** close-to-tray occurs for the first time
- **THEN** the desktop runtime SHALL indicate in the active supported application locale that VaneHub remains available from the system tray
- **AND** an unavailable native locale resolution SHALL use the shared `zh-CN` fallback

### Requirement: Tray window controls
The desktop runtime SHALL provide localized system-tray actions to restore or hide the main window and to quit the application.

#### Scenario: Restore main window
- **WHEN** the user activates the tray show action or tray icon
- **THEN** the runtime SHALL show and focus the existing main window without starting a second application instance

#### Scenario: Hide main window from tray
- **WHEN** the user activates the tray hide action while the window is visible
- **THEN** the runtime SHALL hide the main window without stopping connectors

#### Scenario: Render tray controls in the active locale
- **WHEN** the tray initializes with `zh-CN`, `en`, `zh-TW`, `ja`, or `ko`
- **THEN** show, hide, and quit labels SHALL render in that locale

#### Scenario: Update tray controls after a locale change
- **WHEN** the persisted application language changes while the tray is available
- **THEN** the existing tray controls SHALL update to the new supported locale without restarting the application
- **AND** a failed native label update SHALL preserve the previous usable tray controls and record a redacted warning through unified logging
