## MODIFIED Requirements

### Requirement: IM completion notification
The native chat runtime SHALL expose an event-driven internal terminal completion signal for IM-originated assistant messages and SHALL NOT require fixed-interval database polling to detect completion.

#### Scenario: Assistant completes
- **WHEN** an IM-originated assistant message reaches completed, failed, or cancelled state
- **THEN** the waiting IM job SHALL receive exactly one terminal result associated with the session and assistant message after terminal state persistence

#### Scenario: Assistant terminates before a waiter observes the signal
- **WHEN** terminal state is already persisted before the waiting path begins receiving
- **THEN** the native runtime SHALL return the persisted terminal result without waiting indefinitely or emitting a duplicate result

#### Scenario: Completion waiter is dropped
- **WHEN** an IM job times out, is cancelled, or drops its terminal receiver
- **THEN** the runtime SHALL release completion registration state without retaining an unbounded sender or polling worker

