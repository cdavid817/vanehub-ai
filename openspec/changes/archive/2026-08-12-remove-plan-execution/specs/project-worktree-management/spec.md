## REMOVED Requirements

### Requirement: PlanRun integration worktree isolation
**Reason**: PlanRun preparation and execution are removed, so no Plan-owned worktree is created.
**Migration**: Existing Plan worktrees are left untouched for manual review or cleanup; general and Loop worktree creation remain available.

### Requirement: Serial Plan workspace ownership
**Reason**: There are no Plan SubTask attempts or verification operations after removal.
**Migration**: No replacement is required; retained workflows continue to enforce their own workspace boundaries.

### Requirement: PlanRun worktree review retention
**Reason**: The application no longer owns a live PlanRun lifecycle from which to expose retained worktrees.
**Migration**: Previously created worktrees and branches are not deleted or modified and can be managed with Git tooling.
