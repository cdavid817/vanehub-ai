## ADDED Requirements

### Requirement: Observable operation execution correlation
Observable operation contracts SHALL expose optional execution run and trace correlation for operations that participate in an instrumented task without changing existing operation identity or lifecycle semantics.

#### Scenario: Agent operation starts an execution run
- **WHEN** an Agent generation operation is returned through the service boundary
- **THEN** its contract SHALL expose the associated run id and trace id
- **AND** its existing operation id, kind, status, logs, result, error, and timestamps SHALL remain available

#### Scenario: Legacy operation has no execution run
- **WHEN** an operation does not participate in execution observability
- **THEN** adapters SHALL return absent correlation fields rather than synthesizing false run or trace identities

### Requirement: Shared execution observability contracts
Execution settings, run summaries, span summaries, lifecycle events, fidelity values, and paginated timeline results SHALL participate in the shared Rust and TypeScript contract generation or verification workflow.

#### Scenario: Backend trace contract changes
- **WHEN** a Rust execution-observability command payload or result changes
- **THEN** contract verification SHALL detect drift from the TypeScript service model

#### Scenario: Runtime adapters are tested
- **WHEN** adapter conformance tests run
- **THEN** the Tauri and Web/mock adapters SHALL map execution observability calls, results, pagination, and typed errors according to the same frontend service interface

