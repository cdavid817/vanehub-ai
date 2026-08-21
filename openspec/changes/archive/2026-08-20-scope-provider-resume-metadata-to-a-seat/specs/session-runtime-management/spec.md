## MODIFIED Requirements

### Requirement: Session runtime stores provider resume metadata
The desktop runtime SHALL store provider runtime session metadata when a CLI reports a native session id that can be used for future resume calls. A provider thread belongs to the Agent that created it, so the runtime SHALL scope that metadata to the seat whose generation reported it and SHALL NOT replay it for a seat whose Agent did not create it.

#### Scenario: Capture provider session id
- **WHEN** a provider CLI event includes a native runtime session id for the active generation
- **THEN** the desktop runtime SHALL persist that id against the seat that owns the generation
- **AND** later CLI invocations for that seat SHALL pass that id through the provider-specific resume path when supported

#### Scenario: A seat that has no thread of its own starts one
- **WHEN** a seat takes a turn and no provider runtime session id has been captured for that seat
- **THEN** the desktop runtime SHALL start a new provider thread for it
- **AND** SHALL NOT pass a runtime session id captured for any other seat

#### Scenario: Continue without provider session id
- **WHEN** a provider CLI does not report a native runtime session id
- **THEN** the desktop runtime SHALL continue the current generation without failing solely due to missing resume metadata
- **AND** it SHALL record the missing metadata condition in diagnostics when useful

#### Scenario: A resumed turn fails without producing output
- **WHEN** a turn that passed a stored runtime session id ends failed having produced no output
- **THEN** the desktop runtime SHALL discard that stored id, so the next turn for that seat starts a new provider thread
- **AND** SHALL record the discard in diagnostics
- **AND** the failed turn itself SHALL remain failed rather than being retried

#### Scenario: A resumed turn fails after producing output
- **WHEN** a turn that passed a stored runtime session id produces output and then fails
- **THEN** the desktop runtime SHALL keep that stored id, because output proves the thread was resumed successfully

#### Scenario: Single-seat session keeps its existing thread
- **WHEN** a session created before seats carried their own resume metadata takes a turn
- **THEN** its first seat SHALL resume the runtime session id previously stored for that session
- **AND** the turn SHALL behave as it did before the metadata was scoped to a seat
