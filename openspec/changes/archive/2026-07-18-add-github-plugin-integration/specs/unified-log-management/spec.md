## ADDED Requirements

### Requirement: Plugin integration diagnostics use unified logging
Desktop plugin integration checks SHALL persist diagnostics through the unified logging service with redaction before disk writes.

#### Scenario: Persist plugin readiness diagnostic
- **WHEN** a plugin integration readiness check executes a native command, fails to resolve an executable, times out, or returns an error status
- **THEN** the desktop runtime SHALL write a unified log entry with level, integration id, operation, safe status, and concise redacted message

#### Scenario: Preserve settings feedback
- **WHEN** a plugin integration check produces diagnostic output
- **THEN** the Plugin Integrations settings page SHALL receive a concise service result without reading a feature-local log file

### Requirement: Plugin integration sensitive data redaction
The unified logging service SHALL redact GitHub credentials and host-sensitive data from plugin integration diagnostics before persistence.

#### Scenario: Redact GitHub diagnostic
- **WHEN** GitHub plugin diagnostic output contains a token, bearer value, authorization code, user path, username, email address, or raw protocol output
- **THEN** the persisted entry SHALL omit or replace the sensitive value before it is appended to disk
