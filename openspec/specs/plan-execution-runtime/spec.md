# plan-execution-runtime Specification

## Purpose
TBD - created by archiving change add-plan-execution-foundation. Update Purpose after archive.
## Requirements
### Requirement: Durable Plan execution aggregate
The native runtime SHALL persist PlanRun, SubTaskRun, SubTaskAttempt, verification evidence, control request, and correlation records in SQLite under a dedicated task-orchestration boundary, and SHALL treat persisted state as authoritative over in-memory scheduler state.

#### Scenario: Create execution records from approval
- **WHEN** a valid Plan version is approved
- **THEN** the runtime SHALL create one PlanRun snapshot and one pending SubTaskRun for every snapshotted SubTask in a single consistent operation

#### Scenario: Prevent duplicate dispatch
- **WHEN** overlapping scheduler ticks try to claim the same ready SubTask
- **THEN** a transactional compare-and-set transition SHALL allow at most one dispatch attempt to be created

### Requirement: Deterministic topology-aware serial scheduling
The scheduler SHALL dispatch only SubTasks whose dependencies succeeded, SHALL order eligible work by topological rank, Plan ordinal, and stable SubTask ID, and SHALL run at most one SubTask attempt for a PlanRun at a time in this foundation release.

#### Scenario: Wait for a predecessor
- **WHEN** a pending SubTask has a predecessor that has not reached verified success
- **THEN** the scheduler SHALL NOT dispatch that SubTask

#### Scenario: Select between independent tasks
- **WHEN** multiple independent SubTasks are eligible
- **THEN** the scheduler SHALL dispatch only the deterministic first SubTask and leave the others ready or pending

#### Scenario: Block only descendants after failure
- **WHEN** a required SubTask fails and another unfinished SubTask transitively depends on it
- **THEN** the runtime SHALL mark the descendant blocked while allowing an independent eligible branch to continue serially

### Requirement: Attempt-scoped OnePiece sessions
Each SubTask attempt SHALL execute in a distinct OnePiece API Agent session rooted at the PlanRun integration worktree, SHALL reference the captured OnePiece Profile configuration without copying its credential, and SHALL retain the attempt and session identities after execution stops.

#### Scenario: Dispatch a SubTask attempt
- **WHEN** the scheduler successfully claims an eligible SubTask
- **THEN** it SHALL create a new attempt and API Agent session containing the snapshotted task instructions, acceptance criteria, permitted tools, limits, and bounded context

#### Scenario: Retry creates new evidence
- **WHEN** the user retries an interrupted or failed SubTask through an allowed recovery action
- **THEN** the runtime SHALL create a new attempt and session without overwriting the earlier attempt outcome or evidence

### Requirement: Bounded predecessor context transfer
The runtime SHALL pass only structured summaries and acceptance evidence from direct successful predecessors to a dependent attempt, SHALL order that context deterministically, and SHALL enforce the attempt context budget without including raw predecessor transcripts, prompts, tool arguments, tool results, or credentials.

#### Scenario: Build dependent context
- **WHEN** a SubTask becomes eligible after multiple direct predecessors succeed
- **THEN** the runtime SHALL include their task IDs, bounded result summaries, changed-file summaries, and validation summaries in deterministic predecessor order

#### Scenario: Truncate oversized context
- **WHEN** predecessor context exceeds the configured budget
- **THEN** the runtime SHALL preserve identities, outcomes, acceptance evidence, and truncation metadata before omitting lower-priority descriptive detail

### Requirement: Verification-gated completion
The runtime SHALL execute each required SubTask validation through the guarded operation boundary and SHALL release dependent SubTasks only after the Agent attempt completes and all required verification evidence passes.

#### Scenario: Verification succeeds
- **WHEN** an Agent attempt completes and all declared validation commands pass
- **THEN** the runtime SHALL persist bounded verification evidence, mark the SubTask succeeded, and reevaluate dependent eligibility

#### Scenario: Verification fails
- **WHEN** any required validation command fails or cannot be executed safely
- **THEN** the runtime SHALL record the classified failure, mark the attempt and SubTask failed, and SHALL NOT release its descendants

### Requirement: Plan status projection
The runtime SHALL derive PlanRun status from durable SubTask and control states, SHALL enter awaiting acceptance only after all required SubTasks succeed, and SHALL require explicit user acceptance before marking the PlanRun completed.

#### Scenario: Await final acceptance
- **WHEN** every required SubTask has verified success
- **THEN** the PlanRun SHALL enter `awaiting_acceptance` and expose the retained worktree and evidence for review

#### Scenario: Complete after acceptance
- **WHEN** the user accepts a PlanRun that is awaiting acceptance
- **THEN** the runtime SHALL mark it completed without automatically committing, merging, pushing, or removing its worktree

#### Scenario: Fail exhausted execution
- **WHEN** no runnable SubTask remains and at least one required SubTask is failed or blocked
- **THEN** the runtime SHALL mark the PlanRun failed and retain its worktree, sessions, and evidence

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

### Requirement: Plan execution projects a Run hierarchy
Each PlanRun SHALL correlate to a parent canonical Run and each executing SubTask/Attempt SHALL correlate to a child Run. Pause, retry, verification, cancellation, timeout, and recovery SHALL project canonical transitions while Plan topology and status remain authoritative.

#### Scenario: Plan is cancelled
- **WHEN** a PlanRun is cancelled
- **THEN** its parent Run cancels all non-terminal child Runs and existing Plan cancellation evidence remains compatible

