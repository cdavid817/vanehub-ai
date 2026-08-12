## REMOVED Requirements

### Requirement: Plan execution trace correlation
**Reason**: PlanRun, SubTaskRun, and SubTaskAttempt execution identities are no longer emitted by a live runtime.
**Migration**: Existing session, operation, execution-run, Loop, scheduled-task, delegation, and GroupChat correlations remain unchanged.

### Requirement: Redacted Plan telemetry
**Reason**: Plan-specific lifecycle and telemetry events are removed with task orchestration.
**Migration**: The unified logging and privacy requirements continue to apply to every retained runtime path.
