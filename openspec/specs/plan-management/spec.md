# plan-management Specification

## Purpose
TBD - created by archiving change add-plan-execution-foundation. Update Purpose after archive.
## Requirements
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

### Requirement: Bounded project-aware Plan discovery
The system SHALL let OnePiece gather a bounded project context before producing a Plan draft using only workspace-scoped read, content-search, filename-search, code-search, and configured read-only language-intelligence operations. Discovery SHALL exclude workspace mutations, shell execution, MCP tools, persistent memory writes, credentials, sensitive-file content, and paths outside the canonical project boundary.

#### Scenario: Discover a local project before decomposition
- **WHEN** a user requests a Plan for an available local project
- **THEN** OnePiece SHALL receive bounded discovery limits and SHALL use only the allowed read-only operations before returning the structured Plan draft

#### Scenario: Project index is unavailable
- **WHEN** semantic or local code indexing is unavailable for the selected project
- **THEN** discovery SHALL degrade to the remaining bounded read and search operations and SHALL disclose the degraded planning context on the draft

#### Scenario: Discovery reaches a resource limit
- **WHEN** discovery reaches its tool, token, or time limit
- **THEN** the runtime SHALL stop further discovery and SHALL either generate a draft marked with the bounded-context limitation or return an actionable generation failure

### Requirement: Evidence-linked Plan acceptance policy
Every newly approved Plan SubTask SHALL contain one to three acceptance criteria, each criterion SHALL identify its evidence policy, and every required SubTask SHALL reference at least one required guarded validation command. The Plan SHALL also contain at least one required final validation command covering the integrated result.

#### Scenario: Approve an evidence-bearing Plan
- **WHEN** every SubTask criterion references valid automated or explicit manual evidence and required command references resolve within the Plan version
- **THEN** the Plan MAY be approved after complete graph and verification-policy validation

#### Scenario: Reject vacuous automated verification
- **WHEN** a required SubTask has no required validation command or references an unknown command id
- **THEN** the system SHALL reject approval and identify the affected SubTask and criterion

#### Scenario: Edit verification policy
- **WHEN** a user edits acceptance criteria, validation commands, evidence bindings, retry limits, or final validation commands
- **THEN** the system SHALL create or update a draft version, revalidate the complete Plan, and leave approved execution snapshots unchanged

### Requirement: Immutable execution policy snapshot
Plan approval SHALL snapshot the selected OnePiece Profile reference, discovery limitations, SubTask resource limits, maximum attempt count, eligible automatic-repair classes, criterion evidence bindings, final validation commands, and an optional opaque originating OnePiece session id without copying provider credentials.

#### Scenario: Change defaults after approval
- **WHEN** global planning, retry, Profile, or verification defaults change after a PlanRun is created
- **THEN** the existing PlanRun SHALL continue using its approved non-secret execution-policy snapshot

#### Scenario: Approve from an originating session
- **WHEN** planning began from an identified OnePiece session and the user approves the current Plan version
- **THEN** the PlanRun SHALL retain that session id for bounded association lookup while its execution Attempt sessions remain independently identified

