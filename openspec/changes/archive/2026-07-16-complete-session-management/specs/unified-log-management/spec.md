## ADDED Requirements

### Requirement: Session runtime diagnostics use unified logs
Session-scoped Agent runtime diagnostics SHALL be persisted through the unified logging service in the desktop runtime.

#### Scenario: Persist runtime stdout and stderr
- **WHEN** a session Agent runtime emits stdout, stderr, command diagnostics, or exit status
- **THEN** the native runtime SHALL write those diagnostics through the unified logging service with session id and Agent id context

#### Scenario: Redact runtime diagnostics
- **WHEN** session runtime diagnostics contain sensitive information
- **THEN** the unified logging service SHALL redact sensitive values before writing them to disk

#### Scenario: Keep chat UI separate from diagnostics
- **WHEN** runtime diagnostics are persisted for a session
- **THEN** the chat UI SHALL continue to show user-facing messages through the message service contract
- **AND** raw diagnostic output SHALL NOT be exposed in React components by bypassing the service boundary

#### Scenario: Persist unsupported runtime diagnostics
- **WHEN** a selected Agent CLI is unavailable, not installed, unsupported, exits unsuccessfully, or is cancelled
- **THEN** the native runtime SHALL persist detailed diagnostics through unified logging
- **AND** the user-facing chat error SHALL remain concise
