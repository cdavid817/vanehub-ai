## ADDED Requirements

### Requirement: Prompt Hook diagnostics use unified logging
Prompt Hook execution diagnostics SHALL be persisted through the unified logging service with redaction before disk writes.

#### Scenario: Persist Prompt Hook trace diagnostics
- **WHEN** desktop Prompt Hook assembly completes, skips hooks, disables hooks, or fails
- **THEN** the native runtime SHALL write redacted diagnostic entries through the unified logging service with session id, agent id, hook id, hook status, content hash, token estimate, and safe reason codes when available

#### Scenario: Redact Prompt Hook content
- **WHEN** Prompt Hook diagnostics are persisted
- **THEN** the persisted log entry SHALL NOT contain raw user prompt text, full rendered hook content, effective prompt content, credentials, token-like values, or secret-like values

#### Scenario: Keep settings trace separate from logs
- **WHEN** Prompt Hook trace summaries are displayed in settings
- **THEN** the settings page SHALL receive them through the service boundary
- **AND** React components SHALL NOT read unified log files directly
