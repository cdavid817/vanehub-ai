## MODIFIED Requirements

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

## ADDED Requirements

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
