## MODIFIED Requirements

### Requirement: Pause, cancellation, and restart recovery
The runtime SHALL provide phase-boundary pause, immediate cancellation, and conservative recovery for interrupted Loop runs by consuming the shared recovery projection of each owned Worker or Verifier session.

#### Scenario: Pause after current step
- **WHEN** a user requests pause during an active child operation
- **THEN** the runtime SHALL reconcile that operation and SHALL NOT schedule the next phase
- **AND** the run SHALL become paused at a durable boundary

#### Scenario: Stop immediately
- **WHEN** a user stops an active run
- **THEN** the runtime SHALL request cancellation of the owned Agent or verification process and mark the reconciled run cancelled

#### Scenario: Recover a conclusive role session
- **WHEN** startup session reconciliation reports a conclusive terminal outcome for the active Worker or Verifier execution run
- **THEN** the Loop runtime SHALL project that shared outcome to the owning iteration without deriving a conflicting result from message timestamps or diagnostic telemetry

#### Scenario: Recover interrupted run
- **WHEN** application startup finds a nonterminal run whose owned session recovery is action-required, quarantined, or otherwise inconclusive
- **THEN** the runtime SHALL mark it paused with recovery-required detail
- **AND** it SHALL require explicit resume or cancellation rather than assuming a child process survived or redispatching the interrupted role

