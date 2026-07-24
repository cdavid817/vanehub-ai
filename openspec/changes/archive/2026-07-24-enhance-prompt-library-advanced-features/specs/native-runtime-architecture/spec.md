## ADDED Requirements

### Requirement: Native Prompt Hook version persistence
The native runtime SHALL persist Prompt Hook drafts and immutable published versions through additive SQLite migrations owned by `tooling::prompt_hooks`.

#### Scenario: Migrate existing user Hooks
- **WHEN** an existing database is opened after the versioning migration
- **THEN** each existing user Hook SHALL retain its identity, enabled state, bindings, metadata, template, and version as the selected published snapshot
- **AND** the migration SHALL NOT delete or rewrite unrelated application data

#### Scenario: Publish atomically
- **WHEN** a valid draft is published
- **THEN** appending the immutable version, selecting it, and consuming the matching draft revision SHALL succeed or fail in one transaction

#### Scenario: Query bounded history
- **WHEN** the frontend requests one Hook's version history and evaluation summaries
- **THEN** a bounded native command SHALL query through the Prompt Hook application and repository ports
- **AND** the command handler SHALL NOT contain SQL or domain policy

### Requirement: Native Prompt Hook evaluation persistence
The native runtime SHALL persist idempotent safe execution observations and compute bounded version aggregates without loading Prompt or response bodies.

#### Scenario: Record one terminal observation
- **WHEN** `agent_runtime` reports a terminal invocation outcome through the Prompt Hook published API
- **THEN** the Prompt Hook application service SHALL persist at most one observation for each invocation id, Hook id, and version
- **AND** the write SHALL occur outside the Tauri main thread completion path

#### Scenario: Keep evaluation records safe
- **WHEN** evaluation data is stored, queried, or included in unified diagnostics
- **THEN** it SHALL contain only stable ids, version, outcome, elapsed milliseconds, agent id, and timestamps
- **AND** it SHALL omit Prompt bodies, user or model content, raw errors, credentials, command arguments, and session content
