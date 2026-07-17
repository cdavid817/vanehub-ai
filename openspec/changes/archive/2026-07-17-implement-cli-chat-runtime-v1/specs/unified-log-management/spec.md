## ADDED Requirements

### Requirement: CLI chat runtime diagnostics use unified logging
Desktop CLI chat runtime diagnostics SHALL be persisted through the unified logging service with redaction before disk writes.

#### Scenario: Persist CLI chat stdout and stderr diagnostics
- **WHEN** a provider CLI chat invocation emits stdout, stderr, lifecycle, cancellation, timeout, or failure diagnostics
- **THEN** the desktop runtime SHALL write diagnostic log entries through the unified logging service
- **AND** the entries SHALL include session id, agent id, and runtime context where available

#### Scenario: Redact prompt and secrets
- **WHEN** CLI chat runtime diagnostics contain prompt text, token-like values, API keys, bearer tokens, password-like fields, or secret-like fields
- **THEN** the persisted log entry SHALL redact sensitive values before writing to disk
- **AND** command audit logs SHALL avoid storing raw prompt text

#### Scenario: Keep chat output user-facing
- **WHEN** detailed provider diagnostics are written to unified logs
- **THEN** the chat UI SHALL show concise user-facing errors instead of raw stderr dumps
- **AND** successfully streamed assistant text SHALL remain visible in the message list
