## REMOVED Requirements

### Requirement: Durable Plan execution aggregate
**Reason**: The native PlanRun aggregate and its persistence are removed with standalone Plan execution.

**Migration**: Existing session records remain session-owned; no new PlanRun, SubTaskRun, attempt, or Plan evidence records are created.

### Requirement: Deterministic topology-aware serial scheduling
**Reason**: The Plan task graph scheduler is removed.

**Migration**: OnePiece processes user turns within the originating session without Plan-specific task dispatch.

### Requirement: Attempt-scoped OnePiece sessions
**Reason**: PlanRun no longer creates hidden attempt sessions for subtasks.

**Migration**: Continue work in the user-visible OnePiece session after explicitly changing its execution mode.

### Requirement: Bounded predecessor context transfer
**Reason**: There are no Plan subtasks or predecessor attempts after PlanRun removal.

**Migration**: Session conversation and normal context management provide continuity.

### Requirement: Verification-gated completion
**Reason**: PlanRun-specific verification gates are removed with autonomous subtask execution.

**Migration**: OnePiece may run guarded validation during write-capable session turns subject to the effective Agent policy and ordinary tool approvals.

### Requirement: Plan status projection
**Reason**: PlanRun status and final acceptance have no remaining runtime owner.

**Migration**: Use session lifecycle, generation status, and visible chat evidence.

### Requirement: Durable pause, cancellation, timeout, and recovery
**Reason**: These controls govern the retired PlanRun driver and its attempt boundary.

**Migration**: Use the existing session generation stop and recovery behavior for active OnePiece work.

### Requirement: Durable autonomous Plan driver
**Reason**: Autonomous PlanRun scheduling is intentionally removed rather than embedded into conversation mode.

**Migration**: Each write-capable action proceeds through explicit OnePiece session turns; there is no background Plan driver.

### Requirement: Evidence-driven bounded repair loop
**Reason**: Automatic PlanRun repair attempts and their Plan-specific states are removed.

**Migration**: Failures remain visible in the OnePiece session and can be addressed in a subsequent user-directed turn.

### Requirement: Non-vacuous criterion verification
**Reason**: Persisted Plan criteria and evidence bindings are removed with the Plan execution model.

**Migration**: Users review validation output in the OnePiece conversation without a PlanRun acceptance state.

### Requirement: Plan-level final verification
**Reason**: There is no standalone PlanRun completion boundary at which to run Plan-level final commands.

**Migration**: Request final validation in the active OnePiece session under its effective execution policy.

### Requirement: Plan execution projects a Run hierarchy
**Reason**: PlanRun and SubTask attempts no longer exist as Run owners.

**Migration**: OnePiece session generations continue using the ordinary execution observability model without a Plan parent-child hierarchy.
