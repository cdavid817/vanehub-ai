## REMOVED Requirements

### Requirement: Versioned Plan draft model
**Reason**: The Plan execution product surface and its persisted draft model are retired.
**Migration**: None. Existing database rows remain inert for compatibility and are not exposed by the application.

### Requirement: Strict Plan graph validation
**Reason**: No supported workflow creates or approves Plan dependency graphs after Plan execution is removed.
**Migration**: None. Users should decompose work through ordinary Agent sessions or other retained workflows.

### Requirement: OnePiece-generated Plan draft
**Reason**: OnePiece-specific Plan generation is part of the retired Plan execution workflow.
**Migration**: Use a normal OnePiece session for conversational planning; no structured Plan draft replacement is provided.

### Requirement: Human approval gate
**Reason**: The approval gate only governed creation of the retired PlanRun and its integration worktree.
**Migration**: Existing session and Loop confirmation or permission boundaries remain unchanged.
