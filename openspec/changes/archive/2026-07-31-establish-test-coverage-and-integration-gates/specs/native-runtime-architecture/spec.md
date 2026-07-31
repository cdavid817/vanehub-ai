## ADDED Requirements

### Requirement: Critical native path coverage
Native verification SHALL measure production Rust coverage and MUST maintain at least 80% line coverage for the designated Agent startup and terminal-control, MCP routing, and SQLite transaction path groups.

#### Scenario: Measure Agent runtime critical paths
- **WHEN** native coverage runs
- **THEN** the policy SHALL include Agent launch preparation, terminal open or attach, stop, startup failure, and cleanup behavior in the Agent critical-path group

#### Scenario: Measure MCP routing critical paths
- **WHEN** native coverage runs
- **THEN** the policy SHALL include supported routing, protocol forwarding, timeout, process failure, and bounded error behavior in the MCP critical-path group

#### Scenario: Measure SQLite transaction critical paths
- **WHEN** native coverage runs
- **THEN** the policy SHALL include commit, rollback after a partial-write failure, pool contention, and migration compatibility behavior in the database critical-path group

#### Scenario: Coverage policy path is invalid
- **WHEN** a configured critical-path pattern matches no production Rust source
- **THEN** native coverage validation SHALL fail instead of silently treating the group as covered

### Requirement: Native Session and Agent Terminal lifecycle integration
The native test suite SHALL verify the supported Session and Agent Terminal lifecycle across published application/context boundaries with real temporary SQLite persistence and deterministic process doubles.

#### Scenario: Complete native lifecycle
- **WHEN** the integration test creates a Session, opens its Agent Terminal, observes running state, stops the terminal, and deletes the Session
- **THEN** persisted Session state, operation state, terminal registry state, and associated cleanup SHALL remain consistent after every transition

#### Scenario: Agent Terminal startup fails
- **WHEN** the deterministic process double fails while opening the Agent Terminal
- **THEN** the integration test SHALL verify a command-safe failure, failed lifecycle state, persisted redacted diagnostic association, and release of reserved runtime resources

#### Scenario: Lifecycle operation is repeated
- **WHEN** stop or cleanup is requested again after the Agent Terminal has already stopped
- **THEN** the integration test SHALL verify the documented idempotent result without recreating live runtime state

#### Scenario: Native integration remains deterministic
- **WHEN** the lifecycle integration suite runs on a supported CI host
- **THEN** it SHALL NOT require an installed provider CLI, external network service, user credential, persistent user database, or interactive Tauri window

### Requirement: Transaction rollback verification
Every newly covered multi-write SQLite consistency boundary SHALL include a deterministic failure-injection test proving that a failed later write does not leave earlier writes committed.

#### Scenario: Later write fails
- **WHEN** a deterministic SQLite trigger or repository double rejects a later write within one declared transaction
- **THEN** the test SHALL verify that all writes from that transaction are rolled back and pre-existing data remains unchanged

#### Scenario: Transaction succeeds
- **WHEN** all writes in the declared consistency boundary succeed
- **THEN** the test SHALL verify that all related state becomes visible together after commit
