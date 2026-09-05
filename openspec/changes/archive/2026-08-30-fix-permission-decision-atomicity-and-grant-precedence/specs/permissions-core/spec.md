## ADDED Requirements

### Requirement: Remembered grant identity and selection are deterministic

The system SHALL represent each remembered grant with one canonical key consisting of principal, action, resource, scope, and the scope owner. It SHALL enforce at most one effective active revision for that key, SHALL replace a repeated remembered decision for the same key instead of appending an unordered duplicate, and SHALL select applicable grants in the deterministic order exact Session, exact Project, then Global.

`Scope: Once` and `Effect: Ask` MUST NOT be persisted as remembered grants. Grant lookup MUST NOT depend on SQLite row order, insertion order, query plan, row id, or database maintenance.

#### Scenario: Session decision overrides broader grants

- **WHEN** active Session, Project, and Global grants all match the same principal, action, and resource in the current evaluation context
- **THEN** the exact Session grant SHALL be selected
- **AND** repeated executions SHALL produce the same result regardless of the rows' insertion order

#### Scenario: A repeated remembered decision updates one key

- **WHEN** a user remembers a new Allow or Deny for a canonical grant key that already exists
- **THEN** the system SHALL update that key to one higher revision
- **AND** no second effective row for the same canonical key SHALL remain

#### Scenario: Invalid remembered grant shape is rejected

- **WHEN** persistence is asked to store a Once scope, an Ask effect, a Session grant without its session, a Project grant without its project, or a Global grant with a narrower owner
- **THEN** the system SHALL reject the row before it can participate in permission evaluation

### Requirement: Approval resolution persistence is an atomic consistency boundary

The permissions context SHALL persist an immutable approval resolution, its decision audit, and any remembered-grant intent through one explicit atomic repository operation. The transaction SHALL either commit all owned writes or commit none of them.

A remembered-grant intent SHALL remain inactive until the originating approval delivery is acknowledged, and permission evaluation SHALL consult only active grants.

#### Scenario: Grant write fails inside resolution transaction

- **WHEN** a remembered approval resolution encounters a failure while writing its grant intent, resolution row, or audit row
- **THEN** the transaction SHALL roll back all of those writes
- **AND** the requested action SHALL NOT receive an Allow from that failed attempt

#### Scenario: Delivery acknowledgement activates one grant

- **WHEN** the same immutable resolution is acknowledged as delivered more than once
- **THEN** the resolution SHALL remain delivered
- **AND** its remembered grant SHALL become active exactly once with one revision

### Requirement: Principal creation and evaluation failure remain fail-closed under concurrency

The permissions repository SHALL get or create a principal atomically by stable agent id so concurrent first evaluations resolve to the same principal. An internal evaluation failure SHALL never produce Allow and SHALL produce redacted audit evidence when persistence is available or a redacted unified diagnostic when the failure prevents audit persistence.

#### Scenario: Concurrent first evaluation of one agent

- **WHEN** two evaluations concurrently encounter the same previously unseen stable agent id
- **THEN** both SHALL resolve through the same single principal record
- **AND** neither evaluation SHALL degrade solely because a duplicate principal insert lost a race

#### Scenario: Evaluation storage failure

- **WHEN** grant, principal, policy, or audit storage prevents a complete evaluation
- **THEN** the system SHALL fail closed to Ask or Deny and SHALL NOT execute the action
- **AND** it SHALL retain bounded redacted evidence identifying an evaluation error without persisting sensitive tool input
