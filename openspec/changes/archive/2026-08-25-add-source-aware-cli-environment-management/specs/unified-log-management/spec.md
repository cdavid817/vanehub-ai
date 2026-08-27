## MODIFIED Requirements

### Requirement: Unified operation log persistence

The system SHALL persist SDK and CLI operation logs through the unified logging service.

#### Scenario: Persist SDK operation output

- **WHEN** an SDK install, update, rollback, or uninstall operation emits output
- **THEN** the native runtime SHALL write the operation output to the active log directory with SDK operation context

#### Scenario: Persist CLI operation output

- **WHEN** a CLI discovery, source-catalog, action-planning, Doctor, install, upgrade, downgrade, reinstall, uninstall, repair, cancellation, or verification operation emits output
- **THEN** the native runtime SHALL write bounded redacted output with operation id, Agent id, source, action, phase, and terminal outcome context

#### Scenario: Preserve existing operation UI logs

- **WHEN** an SDK or CLI operation emits output
- **THEN** the settings page operation log display SHALL remain available through the frontend service boundary
- **AND** the frontend view SHALL receive only the bounded redacted representation

#### Scenario: CLI output exceeds its budget

- **WHEN** retained CLI output reaches the configured operation budget
- **THEN** the system SHALL write one truncation marker and retain no additional output beyond the budget
- **AND** it SHALL continue safe process draining or termination without deadlock

## ADDED Requirements

### Requirement: CLI log redaction before UI and disk

CLI process output SHALL be redacted before it is placed in observable operation state, returned to the frontend, or persisted to disk.

#### Scenario: Sensitive provider or package-manager output

- **WHEN** CLI output contains password-like values, tokens, API keys, bearer values, cookies, OAuth codes, secret-like environment values, or provider credential patterns
- **THEN** every retained representation SHALL replace the sensitive value with a redacted marker

#### Scenario: Raw output is received by an adapter

- **WHEN** a source or probe adapter receives raw stdout or stderr
- **THEN** raw content SHALL not be written directly to SQLite, an operation DTO, or a log file

### Requirement: CLI lifecycle audit context

The unified log SHALL retain safe structured context sufficient to diagnose lifecycle decisions without persisting command secrets or installer bodies.

#### Scenario: Action plan is prepared

- **WHEN** a CLI plan is successfully prepared or rejected
- **THEN** the log MAY contain operation id, plan id, Agent id, source id, action, safe version, preflight reason, and elapsed time
- **AND** it SHALL omit raw script bodies, credentials, headers, cookies, and secret-bearing environment values

#### Scenario: Action execution terminates

- **WHEN** a CLI mutation succeeds, partially completes, fails, times out, or is cancelled
- **THEN** the log SHALL contain the normalized outcome, phase, safe exit/timeout/cancel metadata, and diagnostic correlation
- **AND** it SHALL not claim rollback unless a source adapter actually performed and verified one
