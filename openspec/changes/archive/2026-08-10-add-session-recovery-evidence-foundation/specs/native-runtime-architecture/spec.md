## MODIFIED Requirements

### Requirement: Native session maintenance jobs
The desktop runtime SHALL run one-shot session recovery reconciliation in Rust after database, unified logging, session repositories, and runtime evidence adapters are initialized, and SHALL run recurring archival and retention maintenance separately.

#### Scenario: Start maintenance jobs
- **WHEN** the desktop runtime initializes successfully
- **THEN** it SHALL attach the runtime and evidence adapters before reconciling interrupted sessions
- **AND** it SHALL NOT classify sessions as orphaned merely because the runtime adapter has not yet been attached

#### Scenario: Reconcile before dependent runtimes
- **WHEN** startup contains ordinary sessions owned or referenced by Plan or Loop execution
- **THEN** ordinary session evidence reconciliation SHALL complete before Plan and Loop project their recovery outcomes

#### Scenario: Start recurring maintenance after recovery
- **WHEN** startup recovery and dependent Plan/Loop projection have completed or safely deferred retryable storage work
- **THEN** Rust SHALL start automatic archival and retention schedules without combining those mutations with recovery decisions

#### Scenario: Hourly automatic archival schedule
- **WHEN** automatic archival is enabled
- **THEN** Rust SHALL schedule a recurring check approximately once per hour while the application remains running

## ADDED Requirements

### Requirement: Recovery consistency boundaries are failure-injected
The native test suite SHALL verify recovery-critical multi-write transactions and conditional publications against deterministic failures and database reopen.

#### Scenario: Fail a later recovery write
- **WHEN** a failure is injected after an earlier write within a recovery-critical transaction
- **THEN** reopening the file-backed test database SHALL show either the complete transaction or none of it, without a partially published recovery decision

#### Scenario: Reopen after a simulated crash point
- **WHEN** execution is interrupted after a durable generation or recovery transition
- **THEN** a newly constructed runtime SHALL reconcile the reopened database idempotently without relying on the previous process's memory
