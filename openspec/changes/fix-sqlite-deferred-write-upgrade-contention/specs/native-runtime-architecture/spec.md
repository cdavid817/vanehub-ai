## ADDED Requirements

### Requirement: SQLite transaction behaviour matches the transaction's shape
The native runtime SHALL open each SQLite transaction with the behaviour its body requires. A transaction that reads and then writes, performs a compare-and-swap, or drives a state transition SHALL take the write lock at the start. A deferred transaction MUST NOT be used for those shapes, because SQLite refuses the read-to-write lock upgrade with `SQLITE_BUSY` without consulting `busy_timeout`, so such a transaction fails under concurrency regardless of how the timeout is configured.

A multi-statement read whose statements must agree with one another SHALL hold one snapshot for its whole life. A single-statement read SHALL NOT open a transaction.

The native runtime MUST NOT widen `busy_timeout`, add an unbounded retry, or insert a fixed delay in place of opening a transaction with the correct behaviour.

#### Scenario: Read-then-write transaction under contention
- **WHEN** two connections each open a transaction that reads a row and then writes it
- **THEN** one SHALL complete and the other SHALL wait for the write lock within the configured busy timeout, and neither SHALL fail with an un-waited lock error

#### Scenario: Multi-statement read across a concurrent commit
- **WHEN** a multi-statement read is in progress and another connection commits between two of its statements
- **THEN** every statement of that read SHALL observe one snapshot, and the read SHALL NOT return a state assembled from two

#### Scenario: Single-statement read
- **WHEN** a repository performs one read statement
- **THEN** it SHALL NOT open a transaction for it

### Requirement: No external work inside a writer reservation
While the native runtime holds a SQLite write lock it SHALL NOT perform filesystem access, network calls, credential-store access, MCP calls, Hook dispatch, WASM execution, sidecar interaction, process spawning, or any wait on user approval. A flow requiring both SHALL perform the external work before opening the transaction, or afterwards under a compensating step.

#### Scenario: External work during a write flow
- **WHEN** a flow must both write to SQLite and perform external work
- **THEN** the external work SHALL happen outside the transaction, and another connection SHALL be able to acquire the write lock while it is in progress

### Requirement: Distinguishable storage failure identities
The native runtime SHALL present contention, storage failure, and a lost compare-and-swap as three distinguishable identities, and each SHALL remain distinguishable after being converted to a string for a caller that reports or logs it.

#### Scenario: Contention is not reported as corruption
- **WHEN** a transaction cannot start because another connection holds the write lock
- **THEN** the caller SHALL receive a contention identity distinct from a storage-failure identity, so a retry policy does not retry corruption

#### Scenario: A lost compare-and-swap is not a database failure
- **WHEN** a compare-and-swap finds the stored revision has moved
- **THEN** the caller SHALL receive a stale-revision identity distinct from both contention and storage failure

### Requirement: Repositories do not construct raw transactions
Native architecture fitness SHALL reject a repository that constructs a SQLite transaction directly instead of using the runtime's published read or write transaction entry points. The migration runner SHALL be the single exemption, because it owns a distinct atomicity protocol and runs before the connection pool is shared.

#### Scenario: Repository opens a raw transaction
- **WHEN** a file under a context's infrastructure layer calls a connection's transaction constructor directly
- **THEN** native architecture fitness SHALL fail with the rule id, file, line, and the entry point the file should use instead

#### Scenario: Migration runner opens a transaction
- **WHEN** the migration runner opens a transaction through its own protocol
- **THEN** native architecture fitness SHALL permit it
