## ADDED Requirements

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
