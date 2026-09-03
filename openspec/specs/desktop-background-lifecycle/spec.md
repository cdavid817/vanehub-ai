# desktop-background-lifecycle

## Purpose

This capability defines desktop close-to-tray behavior and background lifecycle.

## Requirements

Wall time: 1 seconds
Output:

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

### Requirement: Explicit graceful quit
The desktop runtime SHALL provide an explicit tray quit action that stops connector work before process exit.

#### Scenario: Quit from tray
- **WHEN** the user activates the tray quit action
- **THEN** the runtime SHALL stop accepting new connector messages, request graceful shutdown of all connector lifecycle handles with a bounded timeout, and exit the application

#### Scenario: Connector does not stop in time
- **WHEN** a connector exceeds the graceful shutdown timeout
- **THEN** the runtime SHALL record a redacted warning and continue explicit application exit

#### Scenario: Quit with multiple active Agent sessions
- **WHEN** explicit desktop exit begins while multiple Agent sessions own active generations
- **THEN** the runtime SHALL stop new generation admission and settle the active sessions in bounded parallel batches within the shared shutdown deadline
- **AND** one session's cleanup failure SHALL NOT prevent cancellation from being attempted for the remaining active sessions
- **AND** sessions successfully settled during shutdown SHALL NOT be reported as orphaned failed generations on the next launch

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

### Requirement: Tray-background evolution maintenance
While the desktop process remains active in the tray, the runtime SHALL permit due internal Skill-evolution maintenance only when workspace policy, idle gating, budgets, and mutation safety checks pass. Hiding the window MUST NOT weaken automatic-application consent or safety requirements.

#### Scenario: Hidden desktop becomes idle
- **WHEN** the window is hidden to the tray and an enabled workspace has pending evolution work
- **THEN** the runtime may execute a bounded run under the same gates as a visible desktop

#### Scenario: Explicit quit begins
- **WHEN** the user requests graceful application quit
- **THEN** the runtime stops scheduling new evolution stages and checkpoints or recovers in-progress work before exit

#### Scenario: Tray is unavailable
- **WHEN** native tray initialization failed and normal close exits the process
- **THEN** the runtime does not claim that evolution work continues after process exit
