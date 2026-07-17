## ADDED Requirements

### Requirement: Unified local-extension operation logging
Native local-extension installation, lifecycle, health, and self-test operations SHALL write diagnostic and operation events through the unified logging service while retaining task output for page display.

#### Scenario: Record extension operation output
- **WHEN** an extension operation produces progress, warnings, or failures
- **THEN** the system SHALL retain displayable output on its operation task and SHALL persist corresponding `debug`, `info`, `warn`, or `error` events through the unified logger

#### Scenario: Redact extension log data
- **WHEN** extension output contains proxy credentials, tokens, URLs with credentials, usernames, or sensitive managed paths
- **THEN** the native logging boundary SHALL redact sensitive values before persistence

#### Scenario: Avoid feature-local logs
- **WHEN** a local extension is installed, started, stopped, tested, or uninstalled
- **THEN** the implementation SHALL NOT create an extension-specific persistent log file outside the unified log directory
