## ADDED Requirements

### Requirement: Session runtime owns cancellable CLI processes
The desktop runtime SHALL associate each active CLI chat generation with a cancellable child process owned by the session id.

#### Scenario: Start cancellable CLI generation
- **WHEN** a CLI chat generation starts for a session
- **THEN** the runtime SHALL store the active process handle with that session id
- **AND** stopping another session SHALL NOT affect the stored process

#### Scenario: Stop running CLI generation
- **WHEN** the user stops a session with an active CLI generation
- **THEN** the runtime SHALL request termination of that session's child process
- **AND** the active assistant message SHALL be marked `cancelled`
- **AND** already captured assistant content SHALL remain persisted

#### Scenario: Cleanup on archive or delete
- **WHEN** a running CLI session is archived or deleted
- **THEN** the runtime SHALL stop the owned child process before completing the archive or delete operation
- **AND** lifecycle and message state SHALL reflect the stopped or removed session ownership

### Requirement: Session runtime stores provider resume metadata
The desktop runtime SHALL store provider runtime session metadata when a CLI reports a native session id that can be used for future resume calls.

#### Scenario: Capture provider session id
- **WHEN** a provider CLI event includes a native runtime session id for the active generation
- **THEN** the desktop runtime SHALL persist that id with the owning VaneHub session
- **AND** later CLI invocations for the same session SHALL pass that id through the provider-specific resume path when supported

#### Scenario: Continue without provider session id
- **WHEN** a provider CLI does not report a native runtime session id
- **THEN** the desktop runtime SHALL continue the current generation without failing solely due to missing resume metadata
- **AND** it SHALL record the missing metadata condition in diagnostics when useful
