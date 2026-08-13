# im-session-binding-ui Specification

## Purpose

Defines how users securely attach one configured IM direct chat to an existing VaneHub session and manage that attachment from the workspace UI.

## Requirements

### Requirement: Session-level IM surface
The workspace SHALL expose localized IM binding management for the selected session through the session information panel and a responsive session action.

#### Scenario: Display IM tab
- **WHEN** an eligible session is selected in a layout that shows the information panel
- **THEN** the panel SHALL provide an IM tab with binding state, connector health, and available actions

#### Scenario: Open on a narrow layout
- **WHEN** the information panel is unavailable because of responsive layout
- **THEN** the selected session's actions SHALL provide an equivalent entry to the IM binding surface

#### Scenario: No active session
- **WHEN** no session is selected
- **THEN** the IM binding surface SHALL not offer pairing or binding mutation actions

### Requirement: Guided pairing experience
The session IM surface SHALL guide the user from a configured connector selection through short-lived pairing completion without requiring an Agent or project selection.

#### Scenario: Select available connector
- **WHEN** an unbound eligible session opens the IM binding surface
- **THEN** it SHALL list configured connectors with current health and disable pairing actions for connectors that cannot receive pairing commands

#### Scenario: Show pairing code
- **WHEN** pairing begins successfully
- **THEN** the surface SHALL display the one-time code, connector-specific command guidance, expiry, cancel action, and retry action without persisting the code in frontend storage

#### Scenario: Pairing completes
- **WHEN** the external chat consumes the pairing code
- **THEN** the surface SHALL transition to the bound state without requiring a page reload

#### Scenario: Pairing fails or expires
- **WHEN** pairing fails, is cancelled, or expires
- **THEN** the surface SHALL clear the plaintext code and provide a concise localized retry path

### Requirement: Safe binding summary and controls
The session IM surface SHALL show only safe binding metadata and SHALL provide pause, resume, completion-notification, replace, and remove actions with appropriate confirmation.

#### Scenario: Display bound state
- **WHEN** the selected session has a binding
- **THEN** the surface SHALL display the connector identity, binding state, safe timestamp, connector health, and notification preference without showing raw external chat ids, user ids, delivery targets, credentials, or authorization tokens

#### Scenario: Pause and resume
- **WHEN** the user pauses or resumes a binding
- **THEN** only that binding SHALL enter a busy state and the normalized service result SHALL replace the displayed state

#### Scenario: Toggle completion notifications
- **WHEN** the user changes the binding's completion-notification preference
- **THEN** the surface SHALL persist and display the normalized preference with copy explaining that conversation content is not mirrored

#### Scenario: Replace or remove binding
- **WHEN** the user requests a destructive replacement or removal
- **THEN** the surface SHALL require confirmation, identify the affected connector and session safely, and preserve the session and connector configuration

### Requirement: Runtime-neutral binding service
The desktop and Web/mock runtimes SHALL expose equivalent typed service operations and observable binding-state transitions to the session IM UI.

#### Scenario: Desktop binding operation
- **WHEN** the desktop UI performs a binding operation
- **THEN** the Tauri-specific adapter SHALL call the native communications boundary and React components SHALL NOT invoke Tauri directly

#### Scenario: Web binding operation
- **WHEN** the Web/mock UI performs a binding operation
- **THEN** the Web adapter SHALL provide deterministic non-native behavior with the same contract and safe state semantics

### Requirement: Binding UI localization and accessibility
All session IM labels, instructions, states, confirmations, errors, accessible names, and notices SHALL use synchronized zh-CN and en resources and remain keyboard operable.

#### Scenario: Verify locale parity
- **WHEN** automated tests inspect the session IM translation resources
- **THEN** every binding UI key SHALL exist with equivalent meaning in both supported locales

#### Scenario: Operate with keyboard and assistive technology
- **WHEN** a user navigates pairing and binding controls without a pointer
- **THEN** focus order, status announcements, dialogs, and actions SHALL remain understandable and operable
