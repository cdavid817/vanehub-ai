## MODIFIED Requirements

### Requirement: Retained terminal lifecycle
The desktop runtime SHALL retain Agent Terminal processes across session switching and page closure, then stop inactive processes after two hours or during application shutdown, including all terminal-owned background workers.

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

#### Scenario: Usage worker stops before final refresh
- **WHEN** an Agent Terminal process exits or is stopped
- **THEN** the runtime SHALL signal and join its periodic usage worker before starting the final usage refresh
- **AND** an older periodic result SHALL NOT overwrite the final observation

#### Scenario: Shutdown stops processes
- **WHEN** the desktop application shuts down
- **THEN** the native runtime SHALL stop all live Agent Terminal processes and terminal-owned background workers
- **AND** it SHALL wait for child-process cleanup before releasing terminal state
- **AND** it SHALL write redacted shutdown diagnostics through unified logging

### Requirement: Terminal output persistence boundary
The Agent Terminal runtime SHALL persist runtime session ids, reported usage, and redacted run diagnostics but SHALL NOT convert terminal transcript output or empty usage placeholders into chat messages.

#### Scenario: Output is not written as messages
- **WHEN** an Agent Terminal emits stdout or stderr content
- **THEN** the desktop runtime SHALL display the content in the terminal stream
- **AND** it SHALL NOT create or append `messages` rows for that transcript content

#### Scenario: Empty usage does not create a streaming message
- **WHEN** terminal usage polling has not found a non-zero provider observation
- **THEN** the runtime SHALL NOT create an empty streaming assistant message for usage tracking
- **AND** the session SHALL NOT remain in a streaming state because usage is unavailable

#### Scenario: Diagnostics use unified logging
- **WHEN** an Agent Terminal starts, fails, exits, is stopped by idle cleanup, or is stopped during shutdown
- **THEN** the desktop runtime SHALL write redacted diagnostics through the unified logging service
- **AND** it SHALL NOT create feature-local log files
