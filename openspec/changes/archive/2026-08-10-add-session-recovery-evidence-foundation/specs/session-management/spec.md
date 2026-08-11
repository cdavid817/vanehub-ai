## ADDED Requirements

### Requirement: Durable session recovery metadata
The system SHALL persist recovery status, recovery revision, state revision, history revision, and the optional active execution run identifier with each session without replacing its existing lifecycle state.

#### Scenario: Load an existing session after migration
- **WHEN** a session created before durable recovery metadata is loaded after migration and has no unresolved active state
- **THEN** it SHALL remain readable with recovery status `clean`, initialized revisions, and no fabricated active execution run

#### Scenario: List recovery metadata across runtimes
- **WHEN** sessions are listed through either the desktop or Web/mock service adapter
- **THEN** each normalized session record SHALL expose equivalent lifecycle and recovery fields

### Requirement: Deterministic session message order
Every newly persisted session message SHALL receive a stable, monotonically increasing sequence within its owning session, and historical messages SHALL be deterministically backfilled without relying on timestamp uniqueness.

#### Scenario: Order messages with equal timestamps
- **WHEN** two historical messages in the same session have the same creation timestamp
- **THEN** migration SHALL assign a deterministic relative sequence using stable persisted identity as the tie-breaker

#### Scenario: Page messages by durable order
- **WHEN** a caller pages a session transcript after sequencing is available
- **THEN** messages SHALL neither be skipped nor duplicated because multiple records share a timestamp

## MODIFIED Requirements

### Requirement: Startup session state recovery
The desktop runtime SHALL reconcile persisted active session states after application startup by correlating durable business evidence for the owning execution run and SHALL represent ambiguous recovery safety independently from lifecycle.

#### Scenario: Recover orphan running session
- **WHEN** startup recovery finds a session persisted as `starting` or `running` without a live generation handle and finds one conclusive terminal outcome for its active execution run
- **THEN** the runtime SHALL project that outcome to the session lifecycle, clear the active execution claim, preserve partial content and provider runtime session id, and write recovery diagnostics through unified logging

#### Scenario: Recover unfinished assistant message
- **WHEN** startup recovery finds a `pending` or `streaming` assistant message for the active execution run with no conflicting terminal or uncertain tool evidence
- **THEN** the runtime SHALL mark that message interrupted or failed while preserving already persisted content and SHALL return the session to a recovery-clean terminal lifecycle

#### Scenario: Preserve ambiguous active evidence for review
- **WHEN** an orphan active session contains conflicting execution evidence or unfinished tool activity whose effect is not conclusively known
- **THEN** the runtime SHALL preserve the evidence and place the session in `action_required` rather than treating a failed lifecycle projection as proof that no effect occurred
