## REMOVED Requirements

### Requirement: Versioned Plan draft model
**Reason**: Standalone Plan drafts and versions are removed in favor of conversational planning inside a OnePiece session.

**Migration**: Continue planning in the originating OnePiece session; no replacement versioned Plan record is created.

### Requirement: Strict Plan graph validation
**Reason**: The Plan task graph and its approval validator are part of the retired standalone execution system.

**Migration**: Express ordered implementation steps in the OnePiece conversation without converting them into a persisted Plan graph.

### Requirement: OnePiece-generated Plan draft
**Reason**: OnePiece no longer generates a separate persisted Plan draft for Plan Center.

**Migration**: Ask OnePiece to plan within the active session while it is in read-only Plan mode.

### Requirement: Human approval gate
**Reason**: The PlanRun approval gate is removed with standalone Plan execution.

**Migration**: Use the session-scoped `exit_plan_mode` decision when OnePiece requests permission to leave Plan mode; approval changes only future session turns.

### Requirement: Bounded project-aware Plan discovery
**Reason**: Global and project-scoped discovery of standalone Plans is removed with Plan Center.

**Migration**: Find and resume planning through normal session navigation and session history.

### Requirement: Evidence-linked Plan acceptance policy
**Reason**: Acceptance criteria and command bindings existed only to authorize and verify PlanRun task graphs.

**Migration**: Verification instructions may remain conversational context, but they are not persisted as a Plan execution policy.

### Requirement: Immutable execution policy snapshot
**Reason**: There is no Plan approval event or PlanRun for which to snapshot a separate execution policy.

**Migration**: Each OnePiece generation resolves the session's current execution mode and effective Agent policy through the normal service contract.

### Requirement: Global Plan summary discovery
**Reason**: Global Plan summaries existed to populate and reconcile the retired Plan Center.

**Migration**: Use session summaries and session history; no Plan-specific discovery projection remains.

