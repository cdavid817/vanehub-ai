## ADDED Requirements

### Requirement: Operations-owned redacted log query index

The `operations` context SHALL own a rebuildable SQLite query index for already-redacted unified log records used by interactive session-log queries. The index SHALL NOT replace unified log files as the durable persistence, rotation, archival, export, and repair source.

#### Scenario: Index a persisted redacted log

- **WHEN** the unified logging service durably appends a redacted log record
- **THEN** it SHALL publish an already-redacted bounded record to the operations log-index sink
- **AND** the index SHALL persist safe level, category, message, context, timestamp, record identity, source witness, and available execution/session correlation

#### Scenario: Indexing fails after file append

- **WHEN** the redacted log file append succeeds but the query-index write fails
- **THEN** the original log persistence SHALL remain successful
- **AND** the index SHALL report partial coverage and schedule or expose bounded repair without recursively logging through the failed index path

#### Scenario: Feature requests session logs

- **WHEN** a frontend session-log query is received
- **THEN** the command/application path SHALL use the operations-owned query API
- **AND** it SHALL NOT make `workspaces` or a React component the owner of log-file scanning or query semantics

### Requirement: Stable indexed log identity and idempotency

Every indexed log record SHALL have a stable record id and source-file/offset witness, and index insertion SHALL be idempotent.

#### Scenario: Replay the same source record

- **WHEN** background repair encounters a log record already indexed with the same identity and witness
- **THEN** it SHALL preserve one index row and advance the repair checkpoint safely

#### Scenario: Source witness conflicts

- **WHEN** an existing record id is encountered with conflicting normalized content or source witness
- **THEN** the index SHALL preserve the original row, mark coverage partial, and emit a bounded redacted conflict classification

### Requirement: Bounded asynchronous log-index repair

The system SHALL repair or backfill the query index from retained redacted unified log files through backend-managed bounded operations with stable operation ids, checkpoints, cancellation, and progress.

#### Scenario: Upgrade an existing installation

- **WHEN** the application starts with retained unified log files and an empty or incomplete query index
- **THEN** it SHALL keep startup and unrelated operations responsive while a bounded repair operation indexes source records in batches
- **AND** session-log queries SHALL return available rows with `indexing` coverage

#### Scenario: Restart during repair

- **WHEN** the application stops before repair reaches the end of a source file
- **THEN** the next repair SHALL resume from a persisted validated checkpoint
- **AND** it SHALL not duplicate already indexed rows

#### Scenario: Cancel repair

- **WHEN** the repair operation is cancelled
- **THEN** it SHALL preserve committed index rows and checkpoints
- **AND** coverage SHALL remain indexing or partial rather than complete

### Requirement: Log-index rotation, directory, and retention consistency

The query index SHALL remain consistent with unified log rotation, configured directory changes, archival, and retention without scanning the complete log directory for every write.

#### Scenario: Active log rotates

- **WHEN** the active redacted log file is rotated
- **THEN** the index SHALL retain existing row identity and associate future records with the new active source identity
- **AND** the rotation SHALL not invalidate unrelated indexed rows

#### Scenario: Configured log directory changes

- **WHEN** the active log directory changes
- **THEN** new records SHALL be indexed from the new source
- **AND** records from prior directories SHALL be presented only according to retained source availability and explicit coverage rather than silently mixed as a complete corpus

#### Scenario: Source retention expires

- **WHEN** a retained source log is removed or archived beyond the query retention policy
- **THEN** scheduled maintenance SHALL remove or mark corresponding index rows according to the declared retention contract
- **AND** future coverage SHALL expose the oldest queryable boundary

### Requirement: Post-commit bounded live log publication

After an indexed log transaction commits, the operations query service SHALL publish a bounded identifier-only or already-redacted notice for interested frontend adapters.

#### Scenario: Publish a session-correlated record

- **WHEN** an indexed record has a session id
- **THEN** the live notice SHALL include its stable record id, sequence, timestamp, level, and safe correlation needed for query invalidation
- **AND** it SHALL not include unredacted source input, secrets, raw terminal content, source code, prompts, or unrestricted payloads

#### Scenario: Subscriber is slow

- **WHEN** a live subscriber cannot consume notices within the bounded queue capacity
- **THEN** the publisher SHALL preserve logging and indexing throughput
- **AND** the subscriber SHALL receive one gap/invalidation signal rather than an unbounded backlog

### Requirement: Query index redaction equivalence

The query index, repair path, and live publication SHALL consume only data that has passed the same unified redaction policy as the durable log record.

#### Scenario: Repair reads an older redacted record

- **WHEN** repair parses a retained unified log record
- **THEN** it SHALL validate the stored redaction/schema contract before indexing
- **AND** it SHALL not recover or infer an unredacted source value

#### Scenario: Record cannot satisfy the safe schema

- **WHEN** a source record is malformed, oversized, or violates the safe index schema
- **THEN** repair SHALL skip or quarantine it according to bounded policy, mark coverage partial, and record only a safe reason classification
