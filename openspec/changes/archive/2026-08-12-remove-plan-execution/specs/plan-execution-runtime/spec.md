## REMOVED Requirements

### Requirement: Durable Plan execution aggregate
**Reason**: Durable PlanRun, SubTaskRun, Attempt, evidence, and control records no longer have a supported execution workflow.
**Migration**: Existing rows and tables remain inert; no data conversion is performed.

### Requirement: Deterministic topology-aware serial scheduling
**Reason**: The Plan-specific DAG scheduler is retired with Plan execution.
**Migration**: None. Other schedulers and workflow engines retain their own semantics.

### Requirement: Attempt-scoped OnePiece sessions
**Reason**: Plan SubTask workers and their attempt-owned sessions are no longer created.
**Migration**: Use ordinary OnePiece sessions for direct work.

### Requirement: Bounded predecessor context transfer
**Reason**: This context shape existed only between Plan SubTask attempts.
**Migration**: None.

### Requirement: Verification-gated completion
**Reason**: Plan-specific verification evidence and dependency release are retired.
**Migration**: Loop and ordinary operation verification behavior remain available and unchanged.

### Requirement: Plan status projection
**Reason**: PlanRun lifecycle and final acceptance no longer have a product consumer.
**Migration**: Existing PlanRun state remains stored but is not resumed or projected.

### Requirement: Durable pause, cancellation, timeout, and recovery
**Reason**: These controls applied only to the removed Plan scheduler and attempts.
**Migration**: Session, operation, Loop, and scheduled-task controls remain governed by their existing capabilities.
