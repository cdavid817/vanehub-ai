## REMOVED Requirements

### Requirement: Coordination plan graph
**Reason**: The DAG-based Multi-Agent coordination approach is abandoned. No product surface ever consumed the plan model, so the node/prerequisite contract has no remaining consumer.

**Migration**: None. No replacement plan authoring model is provided. Users who need multiple Agents on one problem continue to create and run Sessions individually.

### Requirement: Dependency-aware scheduling
**Reason**: Scheduling only exists to execute coordination plans, which are being retired together with the capability.

**Migration**: None. Scheduled task management and the Loop engineering runtime remain available for their own scheduling needs and are unaffected.

### Requirement: Prerequisite output propagation
**Reason**: Propagating one node's bounded output into a dependent node's prompt is meaningful only inside a coordination plan.

**Migration**: None. Passing context between Agent runs remains a manual, per-Session activity.

### Requirement: Ordered Agent failover
**Reason**: Primary-then-fallback candidate ordering was defined only for coordination nodes; no other execution path declares fallback Agents.

**Migration**: None. Retryable failures in ordinary Agent execution continue to surface to the user, who chooses whether to retry with a different Agent.

### Requirement: Durable coordination lifecycle
**Reason**: The SQLite-backed persistence of plans, runs, node states, attempt history, and bounded outputs exists solely for this capability. The `coordination_runs` table is dropped with it.

**Migration**: None. Existing local `coordination_runs` rows are discarded without export; they record executions of a feature that had no user-facing surface.

### Requirement: Coordination query and cancellation boundary
**Reason**: The four service-boundary methods and their Tauri commands are removed, so the query and cancellation contract has nothing left to describe.

**Migration**: None. `AgentService` retains its session, runtime, loop, and scheduled-task methods; only the coordination methods are withdrawn from both the Tauri and Web adapters.

### Requirement: Safe coordination diagnostics
**Reason**: Coordination-specific redaction and diagnostic bounds are unnecessary once no coordination run can be created.

**Migration**: None. Unified log management continues to govern redaction and bounded diagnostics for all remaining execution paths.
