## ADDED Requirements

### Requirement: Crash recovery reconciles orphan generations
The desktop runtime SHALL reconcile persisted generation state after a crash or unclean shutdown without assuming provider CLI child processes survived.

#### Scenario: Startup detects orphan generation
- **WHEN** the runtime starts and a persisted session is `starting` or `running` but no in-memory generation handle exists for that session
- **THEN** the runtime SHALL treat the generation as orphaned and SHALL NOT attempt to stop an unrelated process

#### Scenario: Mark orphan generation failed
- **WHEN** an orphan generation is recovered
- **THEN** the owning unfinished assistant message SHALL be marked `failed`, the session lifecycle SHALL be set to `failed`, and partial assistant content SHALL remain available

#### Scenario: Preserve resume metadata
- **WHEN** crash recovery updates an orphan session
- **THEN** the runtime SHALL preserve that session's provider runtime session id so a later provider invocation can use the existing resume path when supported

### Requirement: Recovery diagnostics
Crash recovery SHALL persist redacted diagnostics through the unified logging service.

#### Scenario: Log recovered orphan state
- **WHEN** startup recovery mutates an orphan session or message
- **THEN** the runtime SHALL write a unified log entry with session id, agent id when available, previous lifecycle, new lifecycle, and recovery reason
