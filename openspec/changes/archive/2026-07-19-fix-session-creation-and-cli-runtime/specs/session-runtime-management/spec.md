## ADDED Requirements

### Requirement: Codex CLI message routing

The system SHALL route messages sent in Codex CLI sessions to the Codex runtime and SHALL surface either response events or actionable runtime errors in the session.

#### Scenario: User sends a message to Codex

- **WHEN** a session is created with the Codex CLI agent
- **AND** the user sends a chat message
- **THEN** the system SHALL invoke the Codex CLI with the configured profile
- **AND** the session SHALL show Codex output events or a visible error explaining why output could not be produced.

### Requirement: OpenCode CLI output normalization

The system SHALL parse current OpenCode JSON output into visible chat events.

#### Scenario: OpenCode emits current JSON event shape

- **WHEN** OpenCode emits events using fields such as `sessionID`, `part.text`, `step_start`, or `step_finish`
- **THEN** the runtime SHALL normalize them into provider session id, token, and completed chat events.

#### Scenario: OpenCode npm shim is resolved on Windows

- **WHEN** the configured OpenCode executable resolves to a Windows npm shim
- **THEN** the runtime SHALL resolve the real executable before starting chat generation when the real executable is available.

### Requirement: Shell cwd uses command-compatible Windows paths

Interactive shell sessions SHALL strip Windows extended-length prefixes before launching or resetting the shell current directory.

#### Scenario: CMD starts in selected folder

- **WHEN** a session folder resolves to `\\?\D:\cdavid\Documents\code\claude-code`
- **THEN** the shell runtime SHALL launch or reset the shell cwd using `D:\cdavid\Documents\code\claude-code`.
