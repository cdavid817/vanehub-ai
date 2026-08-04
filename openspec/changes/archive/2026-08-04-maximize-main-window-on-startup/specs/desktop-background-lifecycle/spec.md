## ADDED Requirements

### Requirement: Desktop main window starts maximized
The Tauri desktop runtime SHALL open the main application window maximized to the operating system's available work area on a fresh process launch while preserving standard window controls.

#### Scenario: Launch desktop application
- **WHEN** the user starts a new VaneHub desktop process
- **THEN** the main window SHALL initially be maximized
- **AND** the operating-system taskbar and standard minimize, restore, and close controls SHALL remain available

#### Scenario: Restore the maximized window
- **WHEN** the user restores the initially maximized main window
- **THEN** the window SHALL return to the configured bounded restored size
- **AND** the user SHALL be able to resize it subject to the configured minimum dimensions

#### Scenario: Run Web application
- **WHEN** VaneHub runs through the browser Web/mock runtime
- **THEN** the native maximized-window startup behavior SHALL NOT be claimed or emulated through page scaling

#### Scenario: Use floating assistant
- **WHEN** the desktop floating assistant window is created or shown
- **THEN** it SHALL retain its independent compact sizing and placement behavior
