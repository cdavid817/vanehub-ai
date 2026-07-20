## MODIFIED Requirements

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
