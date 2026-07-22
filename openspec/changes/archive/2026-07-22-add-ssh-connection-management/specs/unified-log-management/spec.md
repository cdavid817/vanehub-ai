## ADDED Requirements

### Requirement: SSH credential and test log redaction
The unified logging service SHALL redact SSH connection credentials and sensitive connection-test diagnostics before persistence.

#### Scenario: Redact SSH password diagnostics
- **WHEN** an SSH connection save or test diagnostic contains password plaintext, password-like fields, credential references, or authentication payloads
- **THEN** the persisted log entry SHALL replace the sensitive value with a redacted marker

#### Scenario: Redact SSH key path diagnostics
- **WHEN** an SSH connection test diagnostic contains a configured private key path
- **THEN** the persisted log entry SHALL avoid exposing the full key path

#### Scenario: Redact SSH test output
- **WHEN** an SSH connection test records stdout, stderr, command text, or failure output
- **THEN** the persisted log entry SHALL include only bounded redacted diagnostic summaries
- **AND** it SHALL NOT persist raw password values, raw private key contents, or unbounded remote command output
