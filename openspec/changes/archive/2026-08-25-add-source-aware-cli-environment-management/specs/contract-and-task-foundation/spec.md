## MODIFIED Requirements

### Requirement: Observable operation model

The system SHALL define a common observable operation model for long-running SDK, MCP, Agent, CLI, and workflow operations.

#### Scenario: Operation status requested

- **WHEN** the frontend requests an observable operation by id
- **THEN** the system SHALL return operation kind, lifecycle status, related entity id where available, optional phase, optional bounded progress, cancellability, timestamps, bounded redacted logs or summary, and final result or error when complete

#### Scenario: Operation events emitted

- **WHEN** an observable operation emits phase, progress, log output, completion, partial-completion result, cancellation, or failure
- **THEN** the system SHALL make that update available through the runtime's supported event or polling mechanism

#### Scenario: Existing operation has no progress metadata

- **WHEN** an existing non-CLI operation does not expose phase, unit progress, or cancellation
- **THEN** those optional fields SHALL be absent or null
- **AND** existing lifecycle semantics SHALL remain unchanged

#### Scenario: CLI operation is created

- **WHEN** CLI refresh, planning, lifecycle execution, bulk execution, or Doctor work starts
- **THEN** the operation kind SHALL be `cli`
- **AND** it MAY expose CLI-specific phase and typed terminal result through the common model

## ADDED Requirements

### Requirement: Observable operation cancellation metadata

The operation contract SHALL identify whether cancellation can currently be requested without implying that every external effect is reversible.

#### Scenario: Cancellation is available

- **WHEN** an operation is queued or its active process can be terminated
- **THEN** the operation SHALL expose `cancellable = true`

#### Scenario: Irreversible step cannot be cancelled safely

- **WHEN** the backend cannot safely interrupt the current external step
- **THEN** it SHALL expose `cancellable = false`
- **AND** the UI SHALL not claim that cancelling would roll back completed effects

### Requirement: CLI contract conformance

Rust command DTOs, TypeScript CLI service models, Tauri mappings, Web/mock mappings, and operation result variants SHALL participate in the shared contract verification workflow.

#### Scenario: CLI contract drifts

- **WHEN** an environment, source, plan, bulk plan, diagnostic, operation progress, or result field changes in one runtime layer
- **THEN** contract verification SHALL fail until all required layers agree

#### Scenario: Runtime adapters are tested

- **WHEN** adapter conformance tests run
- **THEN** Tauri and Web/mock adapters SHALL implement compatible CLI list, refresh, prepare/get/execute action, prepare/get/execute bulk action, Doctor, polling, and cancellation behavior
