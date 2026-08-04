## ADDED Requirements

### Requirement: Bounded native IM work ownership
The native communications context SHALL own explicit bounds for admitted pending messages, active IM Agent generations, completion receivers, and retained per-chat lane state.

#### Scenario: IM traffic exceeds native capacity
- **WHEN** inbound traffic across distinct external chats exceeds the configured IM admission bound
- **THEN** the communications runtime SHALL reject excess work through the connector's bounded busy behavior without creating unbounded tasks, blocking workers, or lane entries

#### Scenario: IM work drains
- **WHEN** admitted work reaches a terminal state or is rejected, cancelled, or timed out
- **THEN** its global capacity reservation, completion registration, and idle lane state SHALL be released exactly once

### Requirement: Failure-isolated connector lifecycle coordination
The native communications context SHALL coordinate lifecycle mutations per connector so one connector's slow or failed operation does not corrupt or block unrelated connectors.

#### Scenario: Replace connector runtime
- **WHEN** an enabled connector receives a validated configuration update
- **THEN** the runtime manager SHALL stop and replace the registered adapter through one coordinated operation and SHALL NOT orphan the previous worker

#### Scenario: One connector startup fails
- **WHEN** startup or shutdown of one enabled connector fails
- **THEN** the runtime SHALL continue attempting the requested lifecycle operation for other connectors and SHALL return or log connector-scoped safe outcomes

