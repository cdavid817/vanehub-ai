## REMOVED Requirements

### Requirement: Bounded OnePiece planning requests
**Reason**: OnePiece no longer produces a structured draft for task orchestration.

**Migration**: Plan conversationally in a read-only OnePiece session.

### Requirement: Attempt execution profile
**Reason**: Plan-specific SubTask attempts and their execution profile are removed.

**Migration**: Execute explicit later turns under the session's Agent mode.

### Requirement: OnePiece credential reference isolation
**Reason**: This requirement governed the retired planner and SubTask execution paths.

**Migration**: Existing OnePiece provider credential isolation remains governed by its general configuration and execution contracts.

### Requirement: OnePiece project discovery execution profile
**Reason**: There is no separate task-orchestration discovery generation.

**Migration**: Use the session Plan-mode read-only tool catalog for project discovery.

### Requirement: OnePiece repair execution profile
**Reason**: Automatic PlanRun repair sessions are removed.

**Migration**: Address failures in a subsequent user-directed OnePiece turn.
