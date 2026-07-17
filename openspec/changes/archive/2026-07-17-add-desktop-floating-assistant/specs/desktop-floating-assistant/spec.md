## ADDED Requirements

### Requirement: Opt-in Windows floating assistant lifecycle
The desktop runtime SHALL provide a floating assistant that is disabled by default and SHALL create or destroy its native window when the persisted setting is enabled or disabled.

#### Scenario: Enable the floating assistant
- **WHEN** a user enables the floating assistant in the Windows Tauri runtime
- **THEN** the runtime SHALL persist the setting and show one floating assistant window without requiring an application restart

#### Scenario: Start with the assistant enabled
- **WHEN** VaneHub starts with the floating assistant setting enabled
- **THEN** the runtime SHALL restore one floating assistant window using the persisted configuration

#### Scenario: Disable the floating assistant
- **WHEN** a user disables the floating assistant
- **THEN** the runtime SHALL persist the disabled state, destroy the floating window, and restore normal main-window close behavior

### Requirement: Native floating-window behavior
The Windows floating assistant SHALL be an undecorated, transparent, always-on-top, taskbar-skipping Tauri window with compact collapsed, menu, and chat sizes.

#### Scenario: Show the collapsed surface
- **WHEN** the floating assistant is enabled and no expanded surface is selected
- **THEN** the native window SHALL occupy only the compact interactive area and SHALL render the Bot/Sparkles control and a non-color-only lifecycle status

#### Scenario: Expand and collapse the native surface
- **WHEN** a user opens the quick menu or mini chat and later collapses it
- **THEN** the runtime SHALL resize and reposition the native window while preserving its desktop anchor and keeping the entire active surface inside a monitor work area

#### Scenario: Restore an invalid saved position
- **WHEN** the saved monitor is unavailable or the saved anchor is outside all current work areas
- **THEN** the runtime SHALL clamp or relocate the assistant to a visible default position on an available monitor

#### Scenario: Drag the assistant
- **WHEN** a user drags the collapsed assistant to another valid desktop position
- **THEN** the runtime SHALL update the native position and persist a monitor-aware anchor for the next application start

### Requirement: Main-window hide and explicit-exit semantics
The desktop runtime SHALL keep the application available through the floating assistant when an enabled main window is closed and SHALL preserve an explicit way to terminate the process.

#### Scenario: Close the enabled main window
- **WHEN** the main window receives a close request while the floating assistant is enabled and can be shown
- **THEN** the runtime SHALL prevent destruction, hide the main window, and keep the floating assistant and active generations running

#### Scenario: Floating window cannot be ensured
- **WHEN** the main window receives a close request but the enabled floating window cannot be created or shown
- **THEN** the runtime SHALL NOT leave a headless inaccessible process and SHALL record the failure through unified redacted logging

#### Scenario: Explicitly exit from the quick menu
- **WHEN** a user activates the translated Exit VaneHub action
- **THEN** the runtime SHALL terminate the application instead of converting the action into another main-window hide

### Requirement: Floating assistant quick actions
The quick menu SHALL provide translated actions to create a new session, return to the active session, open mini chat, open settings, and exit VaneHub.

#### Scenario: Create a new session
- **WHEN** a user selects New Session from the floating menu
- **THEN** the runtime SHALL show, unminimize, and focus the main window and the main surface SHALL open the existing complete create-session dialog

#### Scenario: Return to the active session
- **WHEN** a user selects Return to Current Session
- **THEN** the runtime SHALL show, unminimize, and focus the main workspace on the persisted active session

#### Scenario: Open settings
- **WHEN** a user selects Settings from the floating menu
- **THEN** the runtime SHALL show, unminimize, and focus the main window and navigate it to the settings center

### Requirement: Active-session mini chat
The floating assistant SHALL provide a mini-chat surface for the persisted active session without exposing provider, model, permission-mode, or reasoning controls.

#### Scenario: Open mini chat with an active session
- **WHEN** a user opens mini chat and an active non-archived session exists
- **THEN** the surface SHALL display the session title, lifecycle, messages, thinking/tool/error states, input, send, stop, collapse, and return-to-main controls
- **AND** messages sent from mini chat SHALL use the session's persisted effective chat configuration

#### Scenario: Open mini chat without an active session
- **WHEN** no active usable session exists
- **THEN** the surface SHALL show a localized empty state and a New Session action instead of an enabled message input

#### Scenario: Stream a response while the main window is hidden
- **WHEN** a mini-chat message starts a generation while the main window is hidden
- **THEN** token, thinking, tool, completion, failure, and cancellation events SHALL continue updating the mini-chat surface

#### Scenario: Stop generation from mini chat
- **WHEN** a user activates Stop while the active session is generating
- **THEN** the system SHALL stop that session through the existing agent service and synchronize the cancelled state to both windows

### Requirement: Cross-window state consistency
The desktop runtime SHALL synchronize committed active-session, session-configuration, application-setting, lifecycle, and message changes across main and floating windows.

#### Scenario: Switch sessions in the main window
- **WHEN** the active session changes in the main window
- **THEN** the floating surface SHALL invalidate stale state and render the newly persisted active session and status

#### Scenario: Prevent concurrent generation
- **WHEN** either window attempts to send a second message while the same session already has an active generation
- **THEN** the native service SHALL reject the second send before inserting duplicate user or assistant messages
- **AND** both surfaces SHALL continue displaying the existing generation

#### Scenario: Change language or theme
- **WHEN** the application language or visual style changes in the main settings window
- **THEN** the visible floating surface SHALL update to the committed language and semantic theme without restarting the application

### Requirement: Runtime adapter isolation
Floating-assistant React components SHALL use a frontend service interface, and native window or SQLite operations SHALL remain in Tauri-specific adapters and Rust.

#### Scenario: Invoke a native window action
- **WHEN** a floating-assistant component requests dragging, resizing, main-window restoration, or exit
- **THEN** it SHALL call the floating-assistant service and SHALL NOT directly import or invoke a Tauri API

#### Scenario: Render Basic Configuration in Web runtime
- **WHEN** VaneHub runs through the Web/mock adapter
- **THEN** the adapter SHALL remain interface-compatible and the settings UI SHALL clearly report that the native Windows floating window is unavailable

#### Scenario: Preview the reusable surface in Web runtime
- **WHEN** automated browser tests load the floating-assistant surface directly
- **THEN** the Web adapter SHALL provide deterministic session, configuration, message, and action behavior without claiming native always-on-top support

### Requirement: Localized and accessible floating interactions
The floating assistant SHALL expose synchronized Simplified Chinese and English text and accessible keyboard/focus semantics for every visible action and status.

#### Scenario: Render either supported language
- **WHEN** the active language is `zh-CN` or `en`
- **THEN** menu actions, chat labels, empty states, errors, lifecycle labels, tooltips, and accessible names SHALL render from matching translation resources

#### Scenario: Operate without pointer-only access
- **WHEN** a keyboard user navigates the collapsed control, quick menu, or mini chat
- **THEN** interactive controls SHALL have visible focus, translated accessible names, predictable activation, and Escape behavior appropriate to the current surface mode
