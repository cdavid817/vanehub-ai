## ADDED Requirements

### Requirement: Bounded OnePiece planning requests
The OnePiece native runtime SHALL support a versioned, tool-less planning request that uses the active Profile, receives a bounded structured schema and execution-tool descriptions, and returns content for strict task-orchestration validation without creating execution worktrees or Worker sessions.

#### Scenario: Invoke the planner with an active Profile
- **WHEN** task orchestration requests a Plan draft and OnePiece has an active ready Profile
- **THEN** the runtime SHALL capture that Profile's provider configuration for the generation and SHALL execute no tools during the planning request

#### Scenario: Reject planning without readiness
- **WHEN** task orchestration requests planning but no active OnePiece Profile is ready
- **THEN** the runtime SHALL return an actionable readiness error without starting provider generation or mutating Plan execution state

### Requirement: Attempt execution profile
The OnePiece native runtime SHALL accept an attempt-scoped execution profile containing a bounded root, versioned task instructions, permitted tool catalog, tool-call limit, token budget, and timeout, and SHALL correlate the resulting generation with the supplied PlanRun, SubTaskRun, and Attempt identities.

#### Scenario: Start an attempt generation
- **WHEN** task orchestration starts a valid SubTask attempt
- **THEN** OnePiece SHALL execute through the existing API process gateway using the captured active Profile and the attempt's bounded workspace and limits

#### Scenario: Enforce an attempt limit
- **WHEN** an attempt reaches a configured tool-call, token, or timeout boundary
- **THEN** the runtime SHALL stop at the nearest safe execution boundary and return a classified limit outcome to task orchestration

### Requirement: OnePiece credential reference isolation
Planner and SubTask execution SHALL reuse Profile-scoped OnePiece credentials through the existing credential boundary and SHALL NOT copy credential values into Plan records, task prompts, Agent session metadata, operation metadata, or execution telemetry.

#### Scenario: Persist orchestration metadata
- **WHEN** the runtime stores a planner call or SubTask attempt
- **THEN** it SHALL retain only safe Profile and generation references needed for audit and SHALL keep the credential in its existing secure store

