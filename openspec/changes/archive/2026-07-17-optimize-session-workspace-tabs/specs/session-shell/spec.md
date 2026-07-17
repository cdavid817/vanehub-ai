## ADDED Requirements

### Requirement: Desktop PTY shell
The Shell tab SHALL provide one real PTY-backed interactive shell for the mounted selected-session panel in the desktop runtime.

#### Scenario: Create a shell
- **WHEN** a desktop user first activates Shell for a session with an available root
- **THEN** the UI SHALL show connecting while creation is pending, and the native runtime SHALL start the platform default shell in the canonical session root and return a shell id with connected state after startup succeeds

#### Scenario: Reject executable selection
- **WHEN** the frontend creates a Shell
- **THEN** it SHALL provide the session id and terminal dimensions but SHALL NOT supply an arbitrary executable or working directory

#### Scenario: Receive shell output
- **WHEN** the PTY produces output or changes lifecycle state
- **THEN** the Tauri adapter SHALL route the shell-id and session-id-scoped event through the service boundary to the owning Shell panel

### Requirement: Interactive shell input and resize
The desktop Shell SHALL forward terminal input and terminal dimensions to the owning PTY.

#### Scenario: Enter terminal input
- **WHEN** the user types or sends a control sequence in the xterm view
- **THEN** the service SHALL write those bytes to the matching PTY input

#### Scenario: Resize active terminal
- **WHEN** the visible Shell panel size changes
- **THEN** the frontend SHALL fit the terminal and the native runtime SHALL update the PTY rows and columns

#### Scenario: Return to kept-alive Shell
- **WHEN** the user leaves and returns to Shell without changing sessions
- **THEN** the existing xterm instance and PTY SHALL remain connected and be refitted to the visible panel

### Requirement: Shell controls and status
The Shell tab SHALL expose localized connection status, a return-to-session-directory action, a clear-display action, and a disconnect action.

#### Scenario: Return to session directory
- **WHEN** the user activates CD
- **THEN** the service SHALL send a safely encoded command that changes the existing shell to its canonical session root

#### Scenario: Clear terminal display
- **WHEN** the user activates Clear
- **THEN** the frontend SHALL clear displayed terminal content without claiming to erase native command history

#### Scenario: Disconnect shell
- **WHEN** the user disconnects the shell
- **THEN** the native runtime SHALL terminate the child idempotently and the UI SHALL show a disconnected state

### Requirement: Shell resource cleanup
The system MUST terminate managed PTY children when their owning session workspace can no longer retain them.

#### Scenario: Switch sessions
- **WHEN** the active session changes and the old mounted tab set is reset
- **THEN** the old session Shell SHALL be killed and its frontend subscription SHALL be removed

#### Scenario: Archive or delete session
- **WHEN** a session with a managed Shell is archived or deleted
- **THEN** the native runtime SHALL kill that Shell before completing lifecycle cleanup

#### Scenario: Exit application
- **WHEN** the desktop application exits
- **THEN** the Shell manager SHALL attempt to terminate every managed child

#### Scenario: Repeated kill
- **WHEN** cleanup requests kill for an already exited or previously killed shell
- **THEN** the operation SHALL succeed idempotently without affecting another shell

### Requirement: Shell diagnostic policy
The desktop runtime SHALL persist redacted Shell lifecycle diagnostics but SHALL NOT persist raw interactive commands or raw PTY output as diagnostic logs.

#### Scenario: Shell lifecycle failure
- **WHEN** Shell creation, input, resize, or termination fails
- **THEN** the native runtime SHALL write a redacted error or warning through unified logging with session and shell context

#### Scenario: Shell emits user content
- **WHEN** the user enters a command or the PTY emits output
- **THEN** that raw content SHALL remain page-visible and SHALL NOT be copied into persistent diagnostic logs

### Requirement: Web simulated shell
The Web/mock adapter SHALL provide a deterministic, clearly labelled Shell simulation without starting a local process.

#### Scenario: Open Shell in Web mode
- **WHEN** a Web/mock user activates Shell
- **THEN** the tab SHALL show a simulated connected state and deterministic prompt/output behavior

#### Scenario: Send Web shell input
- **WHEN** input is sent to the simulated shell
- **THEN** the mock SHALL echo or handle supported fixture commands and SHALL identify the output as simulated

#### Scenario: Resize Web shell
- **WHEN** the simulated terminal is resized
- **THEN** the adapter SHALL accept the interface-compatible request without reporting a native PTY side effect
