## ADDED Requirements

### Requirement: Loop run worktree isolation
The desktop runtime SHALL create a dedicated Git worktree and branch for every started Loop run before creating role sessions or modifying project files.

#### Scenario: Prepare Loop worktree
- **WHEN** a valid Loop run starts for a local Git project and base branch
- **THEN** the workspaces context SHALL create a collision-safe Loop branch and sibling worktree through the guarded project operation boundary
- **AND** the run SHALL persist the canonical project path, worktree path, worktree name, and branch

#### Scenario: Reject existing Loop target
- **WHEN** a proposed Loop worktree path or branch conflicts with an existing target
- **THEN** preparation SHALL fail before role-session creation or file mutation
- **AND** concise failure context and detailed redacted diagnostics SHALL remain available

#### Scenario: Use Loop worktree as role root
- **WHEN** Loop worktree preparation succeeds
- **THEN** all Worker and Verifier sessions and verification commands for that run SHALL use the canonical worktree as their bounded root

### Requirement: Loop worktree review retention
The first-phase system SHALL preserve a Loop worktree after success, failure, cancellation, rejection, or restart recovery until a user manages it outside this capability.

#### Scenario: Run reaches terminal state
- **WHEN** a Loop run becomes succeeded, failed, or cancelled
- **THEN** the runtime SHALL retain the worktree and expose its path for review
- **AND** it SHALL NOT automatically execute `git worktree remove`, delete the branch, merge, or commit

