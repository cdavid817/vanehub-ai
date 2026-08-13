## ADDED Requirements

### Requirement: Durable autonomous Plan driver
Starting an approved PlanRun SHALL persist execution intent and activate at most one native driver for that run. The driver SHALL repeatedly claim, execute, verify, and project eligible SubTasks without requiring a browser view, polling callback, or user request between attempts, and SHALL stop at a durable pause, cancellation, action-required, failed, final-review, or completed boundary.

#### Scenario: Continue after verified success
- **WHEN** one SubTask reaches verified success and another SubTask is eligible
- **THEN** the driver SHALL claim and execute the deterministic next SubTask without requiring an “execute next” UI action

#### Scenario: Duplicate driver activation
- **WHEN** startup recovery and a user action concurrently try to activate the same PlanRun
- **THEN** singleton ownership and transactional task claiming SHALL prevent duplicate attempt execution

#### Scenario: Frontend closes during execution
- **WHEN** the desktop frontend closes its Plan view while the native application remains active
- **THEN** the driver SHALL continue according to the persisted PlanRun intent and the frontend SHALL recover progress from bounded service projections when reopened

### Requirement: Evidence-driven bounded repair loop
When an attempt ends with an approved repair-eligible execution or verification failure, the runtime SHALL create a new Attempt and OnePiece session containing bounded sanitized failure evidence and SHALL retry only while the snapshotted maximum attempt count remains. It SHALL retain every earlier session, outcome, changed-file summary, and verification record.

The PlanRun SHALL persist `verifying` while required SubTask checks are executing and `repairing` while an automatic repair Attempt is dispatching or running. These states SHALL be projected consistently after restart and through native and Web/mock service adapters.

#### Scenario: Repair a failed validation
- **WHEN** a required validation command fails, the failure class is repair-eligible, and the attempt budget remains
- **THEN** the runtime SHALL schedule a new Attempt with failed command ids, bounded output summaries, acceptance criteria, and the current changed-file summary
- **AND** the PlanRun SHALL enter `repairing` until the repair reaches its next verification or terminal boundary

#### Scenario: Verify a generated change
- **WHEN** a SubTask Agent generation completes and required guarded checks begin
- **THEN** the PlanRun SHALL enter `verifying` and SHALL retain that state across bounded projections until verification decides the next transition

#### Scenario: Do not retry unsafe or ambiguous failure
- **WHEN** an attempt is cancelled, violates a safety boundary, loses required credentials, or has inconclusive recovery evidence
- **THEN** the runtime SHALL NOT automatically redispatch it and SHALL expose an action-required or recovery-required state

#### Scenario: Exhaust the attempt budget
- **WHEN** a repair-eligible SubTask fails on its final permitted Attempt
- **THEN** the SubTask and affected descendants SHALL remain stopped with retained evidence and the PlanRun SHALL expose the available user recovery actions

### Requirement: Non-vacuous criterion verification
The runtime SHALL mark a SubTask succeeded only after its Agent generation completes, every required validation command passes, and every acceptance criterion has its declared evidence recorded. An empty command set, unresolved criterion binding, or missing required evidence SHALL NOT count as verified success.

#### Scenario: Reject an empty verification set
- **WHEN** an executable required SubTask reaches verification without a required validation command
- **THEN** the runtime SHALL record a verification-policy failure and SHALL NOT release dependent SubTasks

#### Scenario: Record manual evidence requirement
- **WHEN** a criterion is explicitly classified as requiring manual evidence
- **THEN** the runtime SHALL keep the SubTask or PlanRun awaiting the specified user review rather than silently treating the criterion as passed

### Requirement: Plan-level final verification
After all required SubTasks reach verified success, the runtime SHALL run the snapshotted final validation commands against the retained integration worktree and SHALL enter awaiting acceptance only after all required final checks pass.

#### Scenario: Final verification passes
- **WHEN** all required final commands pass and their bounded evidence is persisted
- **THEN** the PlanRun SHALL enter `awaiting_acceptance` and expose the integrated evidence and retained worktree for review

#### Scenario: Final verification is repairable
- **WHEN** a final validation command fails with a repair-eligible classification and final-repair budget remains
- **THEN** the runtime SHALL create a bounded final-repair Attempt without mutating the approved task graph

#### Scenario: Final verification cannot be repaired
- **WHEN** final validation fails without remaining eligible repair budget
- **THEN** the PlanRun SHALL enter an action-required state and SHALL NOT be accepted as completed
