## ADDED Requirements

### Requirement: Versioned Plan draft model
The system SHALL persist each Plan as versioned definitions containing a user goal, ordered SubTask specifications, one to three acceptance criteria per SubTask, optional resource limits, and normalized dependency edges, and it SHALL keep execution snapshots independent from later draft edits.

#### Scenario: Save a valid Plan draft
- **WHEN** a user saves a Plan containing between one and ten valid SubTasks and an acyclic dependency graph
- **THEN** the native runtime SHALL persist a new draft version with stable Plan, version, SubTask, and dependency identities

#### Scenario: Preserve an executing version
- **WHEN** a user edits a Plan after one of its versions has been approved for execution
- **THEN** the system SHALL create or update a separate draft version without changing the approved PlanRun snapshot

### Requirement: Strict Plan graph validation
The system SHALL reject a Plan version before approval when it has no SubTasks, more than ten SubTasks, missing or excessive acceptance criteria, unknown dependency endpoints, self-dependencies, duplicate dependency edges, or a dependency cycle.

#### Scenario: Reject a cyclic Plan
- **WHEN** a draft contains a dependency path that returns to an earlier SubTask
- **THEN** the system SHALL reject the draft with a validation result that identifies the cycle and SHALL NOT make it approvable

#### Scenario: Reject an invalid dependency reference
- **WHEN** a dependency names a SubTask that does not exist in the same Plan version
- **THEN** the system SHALL reject the draft and SHALL NOT persist the invalid edge as executable state

### Requirement: OnePiece-generated Plan draft
The system SHALL use the configured built-in OnePiece Agent to generate a structured draft from the user's goal using versioned planner instructions that include available execution-tool descriptions, a maximum of ten SubTasks, single-session task granularity, acceptance-criteria guidance, and a dependency schema.

#### Scenario: Generate a valid draft
- **WHEN** the user requests planning while an active OnePiece Profile is ready
- **THEN** the runtime SHALL strictly parse and validate the generated structure and present the resulting Plan version as a draft without starting execution

#### Scenario: Handle invalid planner output
- **WHEN** OnePiece returns malformed or semantically invalid Plan output
- **THEN** the runtime SHALL preserve an actionable generation failure and SHALL NOT create an approved PlanRun or start an Agent task session

### Requirement: Human approval gate
The system SHALL require explicit user approval of a valid Plan version before creating its PlanRun, integration worktree, or SubTask Agent sessions.

#### Scenario: Edit before approval
- **WHEN** a user changes a SubTask description, acceptance criterion, ordering, resource limit, or dependency in a draft
- **THEN** the system SHALL revalidate the complete version and keep it non-executing until the user approves it

#### Scenario: Approve a valid version
- **WHEN** the user explicitly approves the current valid Plan version
- **THEN** the runtime SHALL atomically create an immutable PlanRun snapshot referencing that version and make it eligible for preparation

#### Scenario: Reject execution without approval
- **WHEN** a caller requests execution for a draft or invalid Plan version
- **THEN** the runtime SHALL reject the request without creating a worktree, session, or execution operation

