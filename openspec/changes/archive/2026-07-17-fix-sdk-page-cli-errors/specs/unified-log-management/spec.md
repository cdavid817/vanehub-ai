## ADDED Requirements

### Requirement: CLI refresh failure diagnostics
The system SHALL persist detailed CLI refresh and detection diagnostics through unified log management.

#### Scenario: Persist refresh operation failure
- **WHEN** a CLI detection refresh operation fails, times out, or partially completes with per-CLI errors
- **THEN** the native runtime SHALL write a redacted error or warning log entry to the active log directory with operation id, affected CLI id when available, command context, timeout context when applicable, and user-visible error summary

#### Scenario: Persist per-CLI detection failures
- **WHEN** executable resolution, CLI version probing, npm latest-version lookup, or npm version-list lookup fails for a managed CLI
- **THEN** the native runtime SHALL write a redacted diagnostic log entry with the CLI id, package name, executable name, attempted operation, stdout or stderr when available, and failure reason

### Requirement: CLI package operation failure diagnostics
The system SHALL persist detailed install, upgrade, and downgrade diagnostics for managed CLI npm package operations through unified log management.

#### Scenario: Persist npm install failure details
- **WHEN** a CLI package install, upgrade, or downgrade operation fails for Claude Code, OpenCode, Codex CLI, or Gemini CLI
- **THEN** the native runtime SHALL write a redacted error log entry with operation id, CLI id, package name, target version, npm executable, explicit npm arguments, stdout, stderr, exit status or timeout reason, and sanitized environment context

#### Scenario: Keep concise user-facing package error
- **WHEN** detailed CLI package failure diagnostics are persisted
- **THEN** the frontend SHALL receive a concise user-displayable operation error
- **AND** detailed diagnostics SHALL remain available in the active log directory rather than being exposed by bypassing the service boundary
