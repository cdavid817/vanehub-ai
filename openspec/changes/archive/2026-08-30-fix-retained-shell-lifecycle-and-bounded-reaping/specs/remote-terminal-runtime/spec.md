## ADDED Requirements

### Requirement: Remote Shell route and channel ownership remain until confirmed cleanup

A remote Shell route SHALL be generation-qualified and SHALL remain associated with its retained SSH channel and worker controls while the Shell is `Opening`, `Closing`, `Reaping`, or `CloseFailed`. Routed close MUST NOT remove the route solely because a close call returned or failed.

#### Scenario: Remote channel close times out

- **WHEN** a remote Shell channel does not confirm closure within the bounded close attempt
- **THEN** the route and channel ownership SHALL remain attached to the same generation
- **AND** the Shell SHALL transition to `Reaping` or `CloseFailed`
- **AND** a subsequent retry SHALL route to that same remote runtime rather than falling through to a local runtime

#### Scenario: Delayed completion belongs to an old route generation

- **WHEN** a delayed remote worker or Reaper completion references a route generation that is no longer current
- **THEN** the completion SHALL be classified stale
- **AND** it SHALL NOT remove, close, or publish a terminal event for the current route generation

### Requirement: Remote close is bounded and isolated to one Shell channel

Remote close SHALL stop input, request channel EOF/close, cancel or drain owned workers, and observe completion only within finite injected deadlines. A Shell-level close SHALL release only that Shell channel and MUST NOT close unrelated channels sharing the same SSH transport.

#### Scenario: One pooled transport serves two Shells

- **WHEN** two remote Shells use channels on the same pooled SSH transport and one Shell is closed
- **THEN** only the targeted channel SHALL enter closing/reaping/finalization
- **AND** the other channel SHALL remain usable unless an independently observed transport failure affects it

#### Scenario: Remote worker cannot complete before deadline

- **WHEN** a remote reader or writer worker remains blocked after channel close is requested
- **THEN** the command path SHALL return a typed non-terminal disposition within its deadline
- **AND** worker cancellation/completion ownership SHALL transfer to the retained Reaper
- **AND** no unbounded worker join SHALL execute on the command path
