## ADDED Requirements

### Requirement: Explicit session runtime recovery
The system SHALL provide a runtime-neutral session recovery operation that returns a session whose runtime is stuck or failed to a state that accepts new messages, without starting an Agent process.

#### Scenario: Recover a failed session
- **WHEN** recovery is requested for a session whose lifecycle state is `failed`
- **THEN** the system SHALL cancel any generation lease held for that session and cancel any message left in a streaming state
- **AND** it SHALL set the session lifecycle to `idle`
- **AND** the persisted lifecycle state SHALL be the one surfaces read back, so no surface has to infer recovery from the return value alone

#### Scenario: Recovery does not launch a process
- **WHEN** a session is recovered
- **THEN** the system SHALL NOT start a new Agent process or begin a new generation for that session
- **AND** the session SHALL accept the user's next message through the normal message path

#### Scenario: Recovery is idempotent
- **WHEN** recovery is requested for a session that holds no generation lease and has no streaming message
- **THEN** the operation SHALL succeed and report that nothing was cancelled
- **AND** the session lifecycle SHALL be `idle`

#### Scenario: Recovery refuses archived sessions
- **WHEN** recovery is requested for an archived session
- **THEN** the operation SHALL fail with a diagnostic identifying the session as archived
- **AND** it SHALL NOT change the session's lifecycle state

#### Scenario: Recovery reports its outcome
- **WHEN** a recovery operation completes
- **THEN** it SHALL return the identifiers of the messages it cancelled, whether a process was stopped, and the resulting lifecycle state

#### Scenario: Runtime-neutral recovery contract
- **WHEN** recovery is invoked from React in desktop or Web mode
- **THEN** it SHALL be reached through the frontend Agent service boundary
- **AND** Tauri invocation SHALL remain inside the Tauri adapter while the Web adapter provides contract-compatible behavior

#### Scenario: Recovery is recorded
- **WHEN** a recovery operation runs
- **THEN** the system SHALL record it through the unified logging service with the session identity and the outcome
- **AND** it SHALL NOT write a feature-local log file
