## MODIFIED Requirements

### Requirement: Durable pause, cancellation, timeout, and recovery
The runtime SHALL persist pause and cancellation intent before signaling active work, SHALL enforce configured SubTask and PlanRun time limits, and SHALL project restart recovery from shared durable session terminal evidence rather than independently inferring an outcome from recent message order.

#### Scenario: Pause at an attempt boundary
- **WHEN** a user requests pause while an attempt is active
- **THEN** the runtime SHALL persist the request, allow the active attempt to reach a safe terminal boundary, and SHALL NOT claim another SubTask until resumed

#### Scenario: Cancel an active PlanRun
- **WHEN** a user requests cancellation
- **THEN** the runtime SHALL stop new dispatch, signal the active generation or operation, persist terminal outcomes, and retain the integration worktree for review

#### Scenario: Recover a conclusive child session
- **WHEN** startup session reconciliation reports one conclusive terminal outcome for the execution run owned by an in-flight SubTask attempt
- **THEN** the Plan runtime SHALL project that shared outcome to the attempt without scanning recent messages for a different result

#### Scenario: Recover after process restart
- **WHEN** shared session reconciliation reports action-required, quarantined, or otherwise inconclusive terminal evidence for an in-flight attempt
- **THEN** the runtime SHALL mark the attempt interrupted and the PlanRun recovery-required until the user chooses an allowed recovery action

#### Scenario: Retry creates a new execution identity
- **WHEN** the user retries an interrupted attempt through an allowed recovery action
- **THEN** the runtime SHALL create a new attempt, session, and execution run without overwriting the earlier evidence
