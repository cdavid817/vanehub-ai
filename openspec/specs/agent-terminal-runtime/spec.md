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
The desktop runtime SHALL retain Agent Terminal processes across session switching and page closure, then stop inactive processes after two hours or during application shutdown.

#### Scenario: Switch session keeps process
- **WHEN** the user switches away from a session with a live Agent Terminal process
- **THEN** the process SHALL remain live and associated with that session
- **AND** the next selection of that session SHALL attach to the retained process when it is still live

#### Scenario: Idle timeout stops process
- **WHEN** a retained Agent Terminal process has no attach, input, output, or resize activity for more than two hours
- **THEN** the desktop runtime SHALL stop that process
- **AND** the session SHALL remain resumable through its persisted runtime session id when one is available

#### Scenario: Concurrent open attaches once
- **WHEN** repeated or concurrent open requests target the same session while an Agent Terminal is starting
- **THEN** the desktop runtime SHALL serialize the requests through the retained terminal registry
- **AND** it SHALL spawn at most one live Agent CLI process for that session

#### Scenario: Reattach restores terminal output
- **WHEN** the user returns to a session with a live retained Agent Terminal process
- **THEN** the runtime SHALL replay retained terminal output to the newly attached terminal view
- **AND** the user SHALL see the prior terminal screen content instead of an empty terminal

#### Scenario: Reattach uses fast path
- **WHEN** the user returns to a session with a live retained Agent Terminal process
- **THEN** the application service SHALL attach to the retained process before loading a fresh CLI profile or preparing a process launch
- **AND** the terminal content replay SHALL be available without waiting for a full CLI startup path

#### Scenario: Frontend paints cached content immediately
- **WHEN** the Agent Terminal view remounts for a session with cached terminal output
- **THEN** the frontend SHALL paint the cached terminal output before waiting for the native attach response
- **AND** it SHALL avoid duplicating content when the native retained transcript replay arrives

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

### Requirement: Native Agent terminal diagnostics
The desktop runtime SHALL persist redacted Agent terminal launch diagnostics for both successful starts and startup failures.

#### Scenario: Startup command is recorded
- **WHEN** the desktop runtime starts an Agent terminal process
- **THEN** it SHALL record a redacted startup command log entry associated with the VaneHub session id and Agent id

#### Scenario: Startup failure is recorded
- **WHEN** session validation, Agent lookup, availability validation, CLI profile loading, lifecycle update, invocation construction, wrapper generation, PTY creation, process spawning, reader setup, or writer setup fails
- **THEN** the runtime SHALL record a `session.agent_terminal` failure log entry before returning the error
- **AND** the log SHALL include the VaneHub session id and Agent id
- **AND** sensitive command content SHALL remain redacted before persistence

#### Scenario: Retained terminal attach reports running state
- **WHEN** the desktop runtime attaches to an existing retained Agent terminal process
- **THEN** it SHALL return a terminal session with `running` state
- **AND** it SHALL publish a terminal state event with `running` state
- **AND** the Workspace UI SHALL refresh session state after receiving the attach response or terminal state event

### Requirement: Windows managed CLI executable normalization
The desktop runtime SHALL normalize known Windows package-manager shim executables for managed Agent terminal launches when a concrete package binary can be found.

#### Scenario: Missing managed SDK does not block CLI terminal startup
- **WHEN** a single-Agent CLI session starts an Agent terminal for Claude Code, Codex CLI, or another managed CLI Agent
- **AND** the Agent has a missing managed SDK dependency
- **THEN** the runtime SHALL still load the interactive CLI profile for the selected Agent
- **AND** it SHALL attempt startup through the resolved CLI executable, such as `claude`, `codex`, `gemini`, or `opencode`
- **AND** only CLI executable/profile resolution failures SHALL stop the CLI terminal before process launch

#### Scenario: Known shim has package binary
- **WHEN** a managed CLI executable path points to a Windows `.cmd` or `.ps1` shim for Claude Code, Codex CLI, or OpenCode
- **AND** the corresponding package binary exists next to the shim's `node_modules` installation
- **THEN** the Agent terminal runtime SHALL launch the concrete package binary through the native-owned shell wrapper

#### Scenario: Shim cannot be resolved
- **WHEN** a configured executable is not a known Windows shim or no corresponding package binary exists
- **THEN** the runtime SHALL keep the configured executable unchanged

### Requirement: Web runtime terminal parity
The Web/mock runtime SHALL expose the same Agent Terminal service shape without claiming local CLI process execution.

#### Scenario: Web mock opens terminal
- **WHEN** the app runs in Web mode and an Agent Terminal is opened
- **THEN** the Web adapter SHALL provide deterministic simulated terminal state
- **AND** it SHALL NOT access local executables, SQLite, PowerShell, cmd, or a platform shell

#### Scenario: Web mock preserves session metadata
- **WHEN** the Web mock simulates terminal start or resume
- **THEN** it SHALL preserve session agent id, lifecycle state, and mock runtime session id behavior through the frontend service contract

### Requirement: Creation-time resume id capture
The Agent Terminal runtime SHALL persist a provider resume id for a newly created CLI-backed session as soon as the id is available from the terminal start result or runtime events.

#### Scenario: Start result includes resume id
- **WHEN** an Agent Terminal start for a newly created session returns a runtime session id
- **THEN** the desktop runtime SHALL persist that value on the owning session as the session runtime session id
- **AND** subsequent session list and session detail reads SHALL expose the same value

#### Scenario: Runtime event includes resume id
- **WHEN** an Agent Terminal process emits a runtime session id event after startup
- **THEN** the desktop runtime SHALL persist the latest non-empty value on the owning session
- **AND** the frontend SHALL refresh service-backed session state without writing persistence directly

#### Scenario: Web mock creation resume id
- **WHEN** the Web/mock runtime creates and opens a CLI-backed mock session
- **THEN** it SHALL assign or preserve deterministic mock runtime session id metadata through the Agent service contract

### Requirement: Resume id based terminal restore
The Agent Terminal runtime SHALL use a persisted session runtime session id as the provider resume id when opening a CLI terminal for a session without a retained live process.

#### Scenario: Reopen uses persisted resume id
- **WHEN** a user selects a CLI-backed session whose prior process is closed and whose session record has a runtime session id
- **THEN** the desktop runtime SHALL pass that id to the provider-specific resume invocation for the session's stable agent id
- **AND** the restored CLI process SHALL be associated with the same VaneHub session id

#### Scenario: Retained process takes precedence
- **WHEN** a session has both a retained live terminal process and a persisted runtime session id
- **THEN** the desktop runtime SHALL attach to the retained process
- **AND** it SHALL NOT spawn a provider resume invocation for the same session

#### Scenario: Missing resume id starts fresh
- **WHEN** a CLI-backed session has no retained live process and no persisted runtime session id
- **THEN** the desktop runtime SHALL start a fresh provider CLI process for the session's stable agent id

