## ADDED Requirements

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
The runtime SHALL persist pause and cancellation intent before signaling active work, SHALL enforce configured SubTask and PlanRun time limits, and SHALL classify ambiguous in-flight work after restart as recovery-required rather than silently redispatching it.

#### Scenario: Pause at an attempt boundary
- **WHEN** a user requests pause while an attempt is active
- **THEN** the runtime SHALL persist the request, allow the active attempt to reach a safe terminal boundary, and SHALL NOT claim another SubTask until resumed

#### Scenario: Cancel an active PlanRun
- **WHEN** a user requests cancellation
- **THEN** the runtime SHALL stop new dispatch, signal the active generation or operation, persist terminal outcomes, and retain the integration worktree for review

#### Scenario: Recover after process restart
- **WHEN** startup finds an attempt that was recorded in flight without conclusive terminal session or operation evidence
- **THEN** the runtime SHALL mark the attempt interrupted and the PlanRun recovery-required until the user chooses an allowed recovery action

