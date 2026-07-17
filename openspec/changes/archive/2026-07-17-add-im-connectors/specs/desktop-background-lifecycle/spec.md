## ADDED Requirements

### Requirement: Desktop close-to-tray behavior
The Tauri desktop runtime SHALL keep the VaneHub process and enabled IM connectors running when the user closes the main window.

#### Scenario: Close main window
- **WHEN** the user requests to close the main desktop window without explicitly quitting VaneHub
- **THEN** the runtime SHALL prevent process exit, hide the window, and keep enabled connectors running

#### Scenario: Explain first close-to-tray action
- **WHEN** close-to-tray occurs for the first time
- **THEN** the desktop runtime SHALL provide a localized indication that VaneHub remains available from the system tray

### Requirement: Tray window controls
The desktop runtime SHALL provide system-tray actions to restore or hide the main window.

#### Scenario: Restore main window
- **WHEN** the user activates the tray show action or tray icon
- **THEN** the runtime SHALL show and focus the existing main window without starting a second application instance

#### Scenario: Hide main window from tray
- **WHEN** the user activates the tray hide action while the window is visible
- **THEN** the runtime SHALL hide the main window without stopping connectors

### Requirement: Explicit graceful quit
The desktop runtime SHALL provide an explicit tray quit action that stops connector work before process exit.

#### Scenario: Quit from tray
- **WHEN** the user activates the tray quit action
- **THEN** the runtime SHALL stop accepting new connector messages, request graceful shutdown of all connector lifecycle handles with a bounded timeout, and exit the application

#### Scenario: Connector does not stop in time
- **WHEN** a connector exceeds the graceful shutdown timeout
- **THEN** the runtime SHALL record a redacted warning and continue explicit application exit

### Requirement: Tray initialization fallback
The desktop application SHALL remain closable when native tray initialization is unavailable.

#### Scenario: Tray initialization fails
- **WHEN** the runtime cannot create the system tray integration
- **THEN** the application SHALL preserve normal visible-window close behavior and record a redacted warning

### Requirement: Browser lifecycle separation
The browser Web/mock runtime SHALL NOT claim to provide native tray or background-process behavior.

#### Scenario: Run browser-only UI
- **WHEN** VaneHub runs through the Web/mock adapter
- **THEN** native close-to-tray and process quit actions SHALL be absent or explicitly unavailable

