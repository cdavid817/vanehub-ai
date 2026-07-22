## ADDED Requirements

### Requirement: Pooled SQLite connection reuse
The native runtime SHALL serve database operations from a bounded pool of reused, pre-configured SQLite connections instead of opening a new connection per operation, and SHALL initialize schema migration and registry seeding exactly once for the database.

#### Scenario: Reuse connections across operations
- **WHEN** the native runtime performs successive database operations
- **THEN** it SHALL check out an already-open connection from the pool rather than opening a new SQLite connection for each operation
- **AND** each pooled connection SHALL already have busy-timeout, foreign-key enforcement, and write-ahead logging configured

#### Scenario: One-time schema preparation
- **WHEN** the native runtime prepares the database during pool initialization
- **THEN** it SHALL apply versioned migrations and registry seeding exactly once
- **AND** concurrent first-use checkouts SHALL NOT apply migrations or seeding more than once

#### Scenario: Bounded connections under concurrent load
- **WHEN** more concurrent database operations are requested than the pool's maximum size
- **THEN** the runtime SHALL bound the number of live SQLite connections to the configured maximum
- **AND** excess requests SHALL wait for an available connection or fail with a structured timeout error rather than opening unbounded connections
