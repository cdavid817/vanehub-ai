## MODIFIED Requirements

### Requirement: Agent terminal registry cleanup
The native runtime SHALL maintain a bounded registry of live Agent Terminal processes and clean it up deterministically.

#### Scenario: One live terminal per session
- **WHEN** a start request targets a session with an existing live Agent Terminal process
- **THEN** the native runtime SHALL attach to the existing process rather than spawning a second Agent CLI process for that session

#### Scenario: Serialize same-session starts
- **WHEN** two start requests for the same session arrive before the first process launch has finished registering
- **THEN** the native runtime SHALL serialize the open-or-attach path through the terminal registry
- **AND** it SHALL create no more than one live Agent Terminal process for that session

#### Scenario: Cleanup idle terminal
- **WHEN** a live Agent Terminal process exceeds the configured two-hour inactivity threshold
- **THEN** the native runtime SHALL stop the process and remove its live registry entry
- **AND** it SHALL preserve persisted session metadata needed for later resume

#### Scenario: Cleanup on shutdown
- **WHEN** the desktop runtime begins application shutdown
- **THEN** it SHALL stop all live Agent Terminal processes before shutdown completes when possible
- **AND** cleanup failures SHALL be logged through unified logging with redaction
