## ADDED Requirements

### Requirement: Single desktop instance
The Tauri desktop runtime SHALL run at most one VaneHub instance per desktop session. A launch requested while an instance is already running SHALL surface the running instance instead of starting a second application process.

#### Scenario: Launch while an instance is already running
- **WHEN** the user launches VaneHub from the desktop icon, a shortcut, or the start menu while a VaneHub instance is already running
- **THEN** the runtime SHALL NOT start a second application process
- **AND** the already-running instance SHALL show, unminimize, and focus its main window

#### Scenario: Launch while the running instance is hidden in the tray
- **WHEN** the user launches VaneHub while the running instance has hidden its main window to the system tray
- **THEN** the already-running instance SHALL restore and focus its main window
- **AND** the running instance SHALL keep its enabled connectors and background work running without restarting them

#### Scenario: Launch after startup registration already started VaneHub
- **WHEN** launch-on-startup has already started VaneHub and the user then launches it manually
- **THEN** the manually launched process SHALL exit without opening a second window
- **AND** the instance started at login SHALL surface its main window

#### Scenario: Helper subprocess is not a duplicate launch
- **WHEN** the desktop runtime starts one of its own helper subprocesses from the same executable
- **THEN** the helper SHALL run to completion without being treated as a duplicate launch
- **AND** the running instance SHALL NOT change its main window visibility or focus in response
