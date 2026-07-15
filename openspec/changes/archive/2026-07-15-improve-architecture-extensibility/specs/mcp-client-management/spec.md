## ADDED Requirements

### Requirement: MCP contract participation
MCP server configuration, status, test result, import, and export models SHALL participate in the shared frontend-backend contract generation or verification workflow.

#### Scenario: MCP model changes
- **WHEN** a backend MCP model used by a Tauri command changes
- **THEN** the matching TypeScript service model SHALL be updated or verified by the contract workflow

### Requirement: Observable MCP connection tests
MCP connection tests SHALL expose observable operation state when a test may exceed a short immediate command response.

#### Scenario: MCP test operation starts
- **WHEN** a user starts a connection test for an MCP server
- **THEN** the system SHALL expose operation status or progress through the MCP service boundary while preserving the existing final test result behavior

#### Scenario: MCP test command audit
- **WHEN** a stdio MCP test starts a configured external command
- **THEN** the native runtime SHALL record a command execution audit entry associated with the test operation
