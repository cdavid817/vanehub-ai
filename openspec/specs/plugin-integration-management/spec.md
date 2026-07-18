# plugin-integration-management Specification

## Purpose
TBD - created by archiving change add-github-plugin-integration. Update Purpose after archive.
## Requirements
### Requirement: Built-in plugin integration catalog
The system SHALL expose a backend-owned plugin integration catalog with a stable built-in `github` integration.

#### Scenario: List built-in GitHub integration
- **WHEN** the plugin integration service lists integrations
- **THEN** it SHALL return a GitHub integration with stable id `github`, localized metadata keys, version metadata, setup guidance, official documentation URL, and current readiness status

#### Scenario: Reject unknown integration action
- **WHEN** a client requests a readiness test for an integration id outside the backend-owned catalog
- **THEN** the native service SHALL reject the request without executing an external command

### Requirement: GitHub CLI readiness detection
The desktop runtime SHALL test the built-in GitHub integration using a backend-owned `gh auth status` command plan without storing GitHub credentials.

#### Scenario: GitHub CLI authenticated
- **WHEN** `gh auth status` completes successfully in the desktop runtime
- **THEN** the service SHALL mark the GitHub integration configured and ready for GitHub CLI-backed workflows

#### Scenario: GitHub CLI missing
- **WHEN** the host cannot resolve the `gh` executable
- **THEN** the service SHALL mark the GitHub integration as missing CLI and return localized setup guidance without attempting another GitHub command

#### Scenario: GitHub CLI unauthenticated
- **WHEN** `gh auth status` completes with an unauthenticated or failed status
- **THEN** the service SHALL mark the GitHub integration not configured and return a concise user-displayable reason

### Requirement: Plugin integration service boundary
The system SHALL expose plugin integration queries and tests through a dedicated frontend service interface with compatible Tauri and Web/mock adapters.

#### Scenario: Desktop plugin integration request
- **WHEN** a React settings component lists or tests plugin integrations in the Tauri runtime
- **THEN** it SHALL call the plugin integration service and the Tauri adapter SHALL invoke declared native commands

#### Scenario: Web plugin integration request
- **WHEN** the Plugin Integrations page runs outside the Tauri desktop runtime
- **THEN** the Web/mock adapter SHALL return deterministic built-in GitHub metadata and SHALL report live readiness checks as unavailable without importing Tauri APIs

### Requirement: Plugin integration diagnostics
Plugin integration readiness checks that execute native commands SHALL persist redacted diagnostics through unified logging.

#### Scenario: Persist GitHub readiness failure
- **WHEN** a GitHub readiness check fails, times out, or cannot resolve the GitHub CLI
- **THEN** the native runtime SHALL write a redacted diagnostic entry with integration id, operation, safe status, and concise failure classification through the unified logging service

#### Scenario: Avoid credential exposure
- **WHEN** GitHub CLI output or errors include tokens, authorization hints, usernames, host paths, or bearer-like values
- **THEN** persisted diagnostics SHALL redact sensitive values before writing to disk and frontend errors SHALL remain concise

