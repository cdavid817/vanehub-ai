# agent-terminal-runtime Specification

## Purpose
TBD - created by archiving change replace-chat-with-agent-terminal-session. Update Purpose after archive.
## Requirements
### Requirement: Session-scoped Agent terminal
The system SHALL provide a session-scoped Agent Terminal runtime for single-Agent CLI sessions through the frontend Agent service boundary.

#### Scenario: Open terminal for selected session
- **WHEN** the user opens a non-archived single-Agent CLI session
- **THEN** the session page SHALL render an Agent Terminal backed by the frontend Agent service
- **AND** React components SHALL NOT call Tauri commands directly

#### Scenario: Reject archived terminal start
- **WHEN** an Agent Terminal start is requested for an archived session
- **THEN** the service SHALL reject the request without launching a CLI process
- **AND** it SHALL return a concise user-displayable failure

### Requirement: Automatic Agent CLI start
The system SHALL automatically start or attach the selected Agent CLI terminal after a single-Agent session is created or selected.

#### Scenario: Start after create session
- **WHEN** the user creates a single-Agent session for a selected stable agent id
- **THEN** the UI SHALL make that session active
- **AND** it SHALL request Agent Terminal startup for that session without requiring a separate launch button

#### Scenario: Attach existing terminal
- **WHEN** the user selects a session that already has a live retained Agent Terminal process
- **THEN** the UI SHALL attach to the existing terminal stream
- **AND** it SHALL NOT spawn a duplicate CLI process for the same session

### Requirement: Native-owned shell wrapper launch
The desktop runtime SHALL launch Agent Terminal CLI processes through native-owned platform shell wrappers.

#### Scenario: Launch on Windows with PowerShell
- **WHEN** the desktop runtime starts an Agent Terminal on Windows and PowerShell is available
- **THEN** it SHALL run a native-generated PowerShell wrapper
- **AND** the wrapper SHALL invoke the resolved CLI executable with distinct validated arguments

#### Scenario: Fallback to cmd on Windows
- **WHEN** the desktop runtime starts an Agent Terminal on Windows and PowerShell is unavailable
- **THEN** it SHALL run a native-generated cmd wrapper
- **AND** it SHALL preserve the same resolved executable, working directory, and argument tokens

#### Scenario: Launch on macOS or Linux
- **WHEN** the desktop runtime starts an Agent Terminal on macOS or Linux
- **THEN** it SHALL run a native-generated wrapper through the user's default shell or a platform default
- **AND** it SHALL preserve the same resolved executable, working directory, and argument tokens

### Requirement: Interactive CLI profile injection
The Agent Terminal runtime SHALL inject only the selected Agent's saved CLI Parameter profile values that apply to the `interactive` launch scope.

#### Scenario: Use interactive profile
- **WHEN** an Agent Terminal starts for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the desktop runtime SHALL load that stable agent id's saved CLI parameter selections
- **AND** it SHALL project only parameters whose launch scope includes `interactive`

#### Scenario: No session-page overrides
- **WHEN** the Agent Terminal starts
- **THEN** model, permission, reasoning, thinking, and streaming values from the removed session-page chat controls SHALL NOT override the saved CLI profile

### Requirement: Runtime session id persistence and resume
The Agent Terminal runtime SHALL persist provider runtime session ids when available and SHALL use them to resume a later CLI process for the same VaneHub session.

#### Scenario: Persist runtime session id
- **WHEN** an Agent Terminal process reports a provider runtime session id
- **THEN** the desktop runtime SHALL persist that id on the owning session
- **AND** later session reads SHALL expose it as the session runtime session id

#### Scenario: Resume after process closed
- **WHEN** the user opens a session with no live Agent Terminal process and a persisted runtime session id
- **THEN** the desktop runtime SHALL build a provider-specific resume invocation for that stable agent id
- **AND** it SHALL pass the persisted runtime session id using native-owned resume tokens

#### Scenario: Start fresh without runtime session id
- **WHEN** the user opens a session with no live Agent Terminal process and no persisted runtime session id
- **THEN** the desktop runtime SHALL start a fresh Agent CLI process for the selected stable agent id

### Requirement: Retained terminal lifecycle
The desktop runtime SHALL retain Agent Terminal processes across session switching and page closure, then stop inactive processes after 30 minutes or during application shutdown.

#### Scenario: Switch session keeps process
- **WHEN** the user switches away from a session with a live Agent Terminal process
- **THEN** the process SHALL remain live and associated with that session
- **AND** the next selection of that session SHALL attach to the retained process when it is still live

#### Scenario: Idle timeout stops process
- **WHEN** a retained Agent Terminal process has no attach, input, output, or resize activity for more than 30 minutes
- **THEN** the desktop runtime SHALL stop that process
- **AND** the session SHALL remain resumable through its persisted runtime session id when one is available

#### Scenario: Shutdown stops processes
- **WHEN** the desktop application shuts down
- **THEN** the native runtime SHALL stop all live Agent Terminal processes
- **AND** it SHALL write redacted shutdown diagnostics through unified logging

### Requirement: Terminal output persistence boundary
The first Agent Terminal version SHALL persist runtime session ids and redacted run diagnostics but SHALL NOT convert terminal transcript output into chat messages.

#### Scenario: Output is not written as messages
- **WHEN** an Agent Terminal emits stdout or stderr content
- **THEN** the desktop runtime SHALL display the content in the terminal stream
- **AND** it SHALL NOT create or append `messages` rows for that transcript content

#### Scenario: Diagnostics use unified logging
- **WHEN** an Agent Terminal starts, fails, exits, is stopped by idle cleanup, or is stopped during shutdown
- **THEN** the desktop runtime SHALL write redacted diagnostics through the unified logging service
- **AND** it SHALL NOT create feature-local log files

### Requirement: Web runtime terminal parity
The Web/mock runtime SHALL expose the same Agent Terminal service shape without claiming local CLI process execution.

#### Scenario: Web mock opens terminal
- **WHEN** the app runs in Web mode and an Agent Terminal is opened
- **THEN** the Web adapter SHALL provide deterministic simulated terminal state
- **AND** it SHALL NOT access local executables, SQLite, PowerShell, cmd, or a platform shell

#### Scenario: Web mock preserves session metadata
- **WHEN** the Web mock simulates terminal start or resume
- **THEN** it SHALL preserve session agent id, lifecycle state, and mock runtime session id behavior through the frontend service contract

