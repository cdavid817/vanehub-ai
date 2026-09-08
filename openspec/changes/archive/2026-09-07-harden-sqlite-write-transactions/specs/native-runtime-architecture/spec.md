## ADDED Requirements

### Requirement: Write transactions acquire the SQLite write lock up front
Every native SQLite transaction that performs any write SHALL begin as an immediate transaction so that the write lock is acquired when the transaction starts. A deferred transaction SHALL only be used for read-only snapshots. When a write transaction contends with another connection, the native runtime SHALL wait up to the configured busy timeout rather than failing immediately, and a read-before-write sequence inside one transaction SHALL NOT be able to fail with a lock-upgrade error.

#### Scenario: Read-then-write under a concurrent commit
- **WHEN** a repository transaction reads a row to validate a revision or state and another connection commits a write after that read began
- **THEN** the transaction's subsequent write SHALL wait for the write lock within the busy timeout and commit, instead of returning a "database is locked" or snapshot error to the caller

#### Scenario: Read-only snapshot
- **WHEN** infrastructure code needs a consistent view across several reads and performs no write
- **THEN** it MAY use a deferred transaction, and that use SHALL be listed with its reason in the architecture fitness allowlist

#### Scenario: A new deferred write transaction is introduced
- **WHEN** production native code opens a transaction that is not the platform write-transaction entry point and is not on the read-only allowlist
- **THEN** the architecture fitness test SHALL fail and name the file and function

#### Scenario: Startup migrations
- **WHEN** versioned migrations run during database initialization before any command or background task is served
- **THEN** their transactions are exempt from this requirement and SHALL be recorded as such in the allowlist
