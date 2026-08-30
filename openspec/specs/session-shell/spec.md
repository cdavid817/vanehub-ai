# session-shell Specification

## Purpose
Defines desktop PTY and Web-simulated shell behavior, lifecycle controls, cleanup, and diagnostic boundaries for a session workspace.
## Requirements
### Requirement: Desktop PTY shell
The Shell tab SHALL provide one real PTY-backed interactive shell for the mounted selected-session panel in the desktop runtime, using a local process for local workspaces and an authenticated SSH PTY channel for bound remote workspaces.

#### Scenario: Create a shell
- **WHEN** a desktop user first activates Shell for a session with an available root
- **THEN** the UI SHALL show connecting while creation is pending, and the native runtime SHALL start the platform default shell in the canonical session root and return a shell id with connected state after startup succeeds

#### Scenario: Create a local shell
- **WHEN** a desktop user first activates Shell for a local session with an available root
- **THEN** the UI SHALL show connecting while creation is pending, and the native runtime SHALL start the platform default shell in the canonical session root and return a shell id with connected state after startup succeeds

#### Scenario: Create a remote shell
- **WHEN** a desktop user first activates Shell for a bound remote session
- **THEN** the UI SHALL show connecting while the remote runtime acquires a trusted authenticated transport and opens a PTY channel in the remote session path
- **AND** it SHALL return a shell id with connected state after startup succeeds

#### Scenario: Reject executable selection
- **WHEN** the frontend creates a local or remote Shell
- **THEN** it SHALL provide the session id and terminal dimensions but SHALL NOT supply an arbitrary executable or working directory

#### Scenario: Receive shell output
- **WHEN** the local PTY or remote PTY channel produces output or changes lifecycle state
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

### Requirement: Remote Shell cleanup preserves transport reuse
Remote Shell cleanup SHALL close the owning PTY channel while allowing a healthy pooled SSH transport to remain available within its idle and capacity limits.

#### Scenario: Switch remote sessions
- **WHEN** the active session changes and the old mounted remote Shell tab is reset
- **THEN** the runtime SHALL close the old PTY channel and release its transport lease
- **AND** it SHALL NOT close a transport that remains leased by another channel

#### Scenario: Exit application
- **WHEN** the desktop application exits
- **THEN** the runtime SHALL close all remote channels and pooled transports

### Requirement: Dedicated Terminal content boundary
The Shell SHALL keep raw interactive input out of persistence while allowing configured normalized output capture in the dedicated Terminal content store.

#### Scenario: Shell emits user content
- **WHEN** the user enters a command or a local or remote PTY emits output
- **THEN** raw input and output SHALL NOT be copied into persistent diagnostic logs
- **AND** eligible remote output MAY be persisted only through the bounded Terminal output service

### Requirement: Shell natural-exit reclamation
The desktop shell runtime MUST remove and reap a managed PTY child after the child exits naturally, and frontend event subscriptions MUST remain cleanup-safe while registration is pending.

#### Scenario: Shell exits without an explicit disconnect
- **WHEN** a local PTY reaches EOF or reports process exit before the user requests disconnect
- **THEN** the runtime SHALL remove the matching shell generation from its live registry and wait for the child without affecting a replacement shell

#### Scenario: Shell view unmounts during subscription
- **WHEN** a Shell view is disposed before asynchronous event subscription completes
- **THEN** the completed subscription SHALL be immediately removed and SHALL NOT deliver events to the disposed terminal

### Requirement: Shell startup reserves capacity and ownership before external execution

The session Shell application service SHALL atomically reserve every applicable capacity limit and SHALL register a generation-qualified Shell in `Opening` before opening a local process or remote channel. Startup SHALL transfer every acquired runtime resource through one launch guard that either commits ownership, confirms rollback, or hands cleanup to a retained Reaper.

#### Scenario: Concurrent opens compete for the last session slot

- **WHEN** two requests concurrently attempt to create different Shells and only one total or per-session capacity slot remains
- **THEN** exactly one request SHALL obtain the capacity reservation
- **AND** the other request SHALL fail with a typed capacity result before spawning a process or opening a remote channel

#### Scenario: Startup fails after the child is created

- **WHEN** a local child is spawned but reader, writer, worker, route, or retained-runtime setup fails before startup commit
- **THEN** the launch guard SHALL retain ownership of every acquired resource
- **AND** it SHALL either confirm cleanup and transition to `OpenFailed` or hand the same generation to the Reaper
- **AND** no child, PTY, channel, worker, or capacity lease SHALL become ownerless

#### Scenario: Shell exits during Opening

- **WHEN** a Shell emits output and exits before startup reaches `Running`
- **THEN** the pre-registered Shell SHALL retain the ordered output and exit evidence
- **AND** a later startup completion SHALL NOT overwrite the terminal state with `Running`

### Requirement: Shell close is bounded, typed, and confirmed before terminal publication

The session Shell close use case SHALL transition an addressable generation through `Closing`, `Reaping`, or `CloseFailed` while runtime cleanup is unconfirmed. It SHALL return within a finite command-path deadline and MUST NOT publish `ShellClosed`, remove lifecycle ownership, release capacity, or report terminal success until the owned process/channel is confirmed terminal.

#### Scenario: Child does not exit within the close deadline

- **WHEN** graceful and forceful close stages cannot confirm child termination within the command-path budget
- **THEN** close SHALL return a typed `Reaping` or `CloseFailed` disposition
- **AND** the same Shell generation, runtime handles, replay state, and capacity lease SHALL remain addressable for retry or Reaper cleanup
- **AND** the system SHALL NOT publish `ShellClosed`

#### Scenario: Reaper later confirms termination

- **WHEN** the retained Reaper confirms termination for the current Shell generation
- **THEN** the system SHALL finalize the terminal state exactly once
- **AND** it SHALL release runtime ownership, route, replay retention according to policy, and capacity exactly once
- **AND** it SHALL publish one terminal event after that finalization

#### Scenario: Close is requested repeatedly

- **WHEN** several callers close a Shell that is already Closing, Reaping, CloseFailed, or terminal
- **THEN** the service SHALL reconcile with the existing generation and close operation
- **AND** it SHALL NOT create competing termination attempts, double-release capacity, or publish duplicate terminal events

### Requirement: Session cleanup reports every Shell outcome

Archive, delete, idle sweep, and application shutdown SHALL consume typed Shell close results and SHALL retain an aggregate cleanup report. They MUST NOT silently discard a close failure or claim a Shell was closed merely because cleanup was requested.

#### Scenario: Session delete encounters a reaping Shell

- **WHEN** session deletion requests closure of all owned Shells and at least one remains `Reaping` or `CloseFailed`
- **THEN** strict deletion SHALL return `session_shell_cleanup_incomplete`
- **AND** the session and Shell identities required for retry and diagnosis SHALL remain available
- **AND** the UI SHALL NOT claim final deletion succeeded

#### Scenario: Idle sweep schedules cleanup

- **WHEN** idle sweep reaches a Shell that cannot close within its bounded attempt
- **THEN** the sweep SHALL record that Shell as reaping or failed rather than closed
- **AND** it SHALL expose bounded structural cleanup evidence without blocking the sweep indefinitely

### Requirement: Frontend and Web adapters expose retained lifecycle semantics

The frontend Shell service, Tauri adapter, and Web/mock adapter SHALL use one typed contract for `Opening`, `Running`, `Closing`, `Reaping`, `CloseFailed`, and terminal outcomes. React components SHALL reconcile events with service reads and SHALL present localized intermediate/failure states without claiming native cleanup that did not occur.

#### Scenario: Close returns Reaping

- **WHEN** the service returns a `Reaping` close disposition
- **THEN** the UI SHALL keep the Shell identifiable and disable or adapt operations that require Running
- **AND** it SHALL show a localized cleanup-in-progress state until pull or event reconciliation produces a terminal or failed result

#### Scenario: Web mode simulates a late old-generation completion

- **WHEN** Web/mock creates a newer generation after an older simulated Shell is closed and then emits a delayed completion for the old generation
- **THEN** the newer generation SHALL remain unchanged
- **AND** capacity and terminal events SHALL be applied only to the matching generation

