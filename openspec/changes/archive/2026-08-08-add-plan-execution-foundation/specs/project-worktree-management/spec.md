## ADDED Requirements

### Requirement: PlanRun integration worktree isolation
The desktop runtime SHALL create one dedicated Git branch and sibling worktree for an approved PlanRun before dispatching its first SubTask and SHALL persist the canonical project path, recorded base OID, branch, worktree name, and worktree path.

#### Scenario: Prepare a PlanRun worktree
- **WHEN** an approved PlanRun enters preparation for a valid Git project
- **THEN** the workspaces context SHALL create a collision-safe Plan branch and worktree through the guarded project operation boundary before any SubTask Agent session starts

#### Scenario: Reject a conflicting target
- **WHEN** the proposed Plan branch or worktree path conflicts with an existing target
- **THEN** preparation SHALL fail safely without dispatching a SubTask or modifying the conflicting target

### Requirement: Serial Plan workspace ownership
Every SubTask attempt and verification operation for a foundation PlanRun SHALL use its canonical integration worktree as the bounded root, and the scheduler SHALL ensure no two attempts mutate that worktree concurrently.

#### Scenario: Start sequential task work
- **WHEN** the scheduler dispatches a SubTask attempt
- **THEN** its Agent session and validation commands SHALL use the persisted PlanRun worktree and no other attempt for that PlanRun SHALL be active

### Requirement: PlanRun worktree review retention
The system SHALL retain the PlanRun worktree after completion, failure, cancellation, rejection, or recovery-required state and SHALL NOT automatically commit, merge, push, reset, remove the worktree, or delete its branch.

#### Scenario: Finish Plan execution
- **WHEN** a PlanRun reaches any terminal state or awaits user acceptance
- **THEN** the runtime SHALL expose the retained worktree path for review without applying its changes to the source or target branch

