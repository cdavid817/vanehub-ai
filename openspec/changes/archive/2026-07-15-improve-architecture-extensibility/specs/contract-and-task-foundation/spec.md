## ADDED Requirements

### Requirement: Shared frontend backend contracts
The system SHALL keep Rust command payload/result models and TypeScript service models synchronized through generated contracts or contract verification.

#### Scenario: Generate command models
- **WHEN** backend command payload or result models change
- **THEN** the project SHALL provide a repeatable way to update matching TypeScript model definitions

#### Scenario: Detect contract drift
- **WHEN** generated or verified TypeScript contracts differ from committed frontend models
- **THEN** the verification workflow SHALL fail until the contract drift is resolved

### Requirement: Typed service errors
Frontend service interfaces SHALL expose errors in a normalized shape that preserves user-displayable messages and machine-readable categories where available.

#### Scenario: Backend validation error
- **WHEN** a backend service rejects input with a validation error
- **THEN** the frontend service adapter SHALL expose a typed error that the UI can display without parsing arbitrary strings

#### Scenario: Runtime unavailable error
- **WHEN** a Web runtime requests a desktop-only operation
- **THEN** the frontend service adapter SHALL expose a typed unsupported-runtime error rather than attempting a Tauri command

### Requirement: Observable operation model
The system SHALL define a common observable operation model for long-running SDK, MCP, Agent, and future workflow operations.

#### Scenario: Operation status requested
- **WHEN** the frontend requests the status of an observable operation by id
- **THEN** the system SHALL return the operation kind, lifecycle status, related entity id where available, progress or log summary where available, and final result or error when complete

#### Scenario: Operation events emitted
- **WHEN** an observable operation emits progress, log output, completion, or failure
- **THEN** the system SHALL make that update available to the frontend through the runtime's supported event or polling mechanism

### Requirement: Adapter conformance
Each runtime adapter for Agent, MCP, SDK, and future workspace services SHALL conform to the same service interface contract for common operations.

#### Scenario: Tauri adapter conformance
- **WHEN** service interface conformance tests run against the Tauri adapter with mocked command responses
- **THEN** the adapter SHALL map service calls, results, and errors according to the shared contract

#### Scenario: Web adapter conformance
- **WHEN** service interface conformance tests run against the Web adapter
- **THEN** the adapter SHALL return contract-compatible mock or HTTP-backed responses without importing Tauri APIs
