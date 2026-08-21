## REMOVED Requirements

### Requirement: Plan execution trace correlation
**Reason**: PlanRun, SubTaskRun, and SubTaskAttempt are retired runtime owners.

**Migration**: Use ordinary session generation and canonical Run correlation.

### Requirement: Redacted Plan telemetry
**Reason**: No live Plan execution path emits Plan-specific diagnostics.

**Migration**: Continue applying metadata-only privacy rules to session and Run observability.

### Requirement: Autonomous Plan loop trace correlation
**Reason**: The background Plan driver, repair chain, and Plan service no longer exist.

**Migration**: Observe user-directed OnePiece turns through existing session and Run telemetry.
