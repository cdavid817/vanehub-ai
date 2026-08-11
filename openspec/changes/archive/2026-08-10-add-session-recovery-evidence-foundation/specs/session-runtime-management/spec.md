## MODIFIED Requirements

### Requirement: Session-scoped runtime ownership
The desktop runtime SHALL correlate active Agent generation state with a stable execution run and session id, SHALL claim that ownership durably before execution, and SHALL prevent unrelated or competing work from sharing the claim or generation handle.

#### Scenario: Start generation for a session
- **WHEN** a message is accepted for a non-archived, recovery-clean session with no active execution run
- **THEN** the desktop runtime SHALL durably associate the execution run with that session before provider or CLI execution
- **AND** the session lifecycle SHALL transition to `starting` and then `running` when execution begins

#### Scenario: Reject duplicate same-session generation
- **WHEN** another submission races to start while the session already owns an active execution run
- **THEN** the durable claim SHALL reject the competing generation without starting another provider request or child process

#### Scenario: Isolate concurrent sessions
- **WHEN** two sessions have independent generation state
- **THEN** stopping, completing, or recovering one session SHALL NOT stop, complete, recover, or mutate the other session's active generation

### Requirement: Crash recovery reconciles orphan generations
The desktop runtime SHALL reconcile persisted generation state after a crash or unclean shutdown without assuming provider API requests, CLI child processes, or provider-internal tool activity survived, completed, or had no side effects.

#### Scenario: Startup detects orphan generation
- **WHEN** the runtime starts and a persisted session is `starting` or `running` but no live generation handle exists for that session
- **THEN** the runtime SHALL enter reconciliation for that session and SHALL NOT attempt to stop an unrelated process

#### Scenario: Reconcile conclusive orphan evidence
- **WHEN** an orphan generation has one conclusive terminal business outcome correlated with its active execution run
- **THEN** the owning assistant message and session lifecycle SHALL reflect that outcome, the active execution claim SHALL be cleared, and partial assistant content SHALL remain available

#### Scenario: Mark orphan generation failed
- **WHEN** an orphan generation has a conclusive failed outcome or an unfinished tool-free assistant response that can be terminated without ambiguity
- **THEN** the owning unfinished assistant message SHALL be marked `failed`, the session lifecycle SHALL be set to `failed`, the active execution claim SHALL be cleared, and partial assistant content SHALL remain available

#### Scenario: Require review for inconclusive activity
- **WHEN** an orphan generation has conflicting evidence or incomplete provider, CLI, or tool activity whose outcome cannot be proven
- **THEN** the session SHALL become recovery-action-required and the runtime SHALL NOT automatically retry or fabricate a terminal tool result

#### Scenario: Preserve resume metadata
- **WHEN** crash recovery reconciles an orphan session
- **THEN** the runtime SHALL preserve that session's provider runtime session id so a later explicit invocation can use the existing resume path when supported
- **AND** preserving the id SHALL NOT automatically resume the interrupted generation
