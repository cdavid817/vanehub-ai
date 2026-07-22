## ADDED Requirements

### Requirement: SSH failure state refresh
The SSH connection settings page SHALL refresh profile data after a connection test settles, regardless of whether the test succeeds or fails.

#### Scenario: Failed test refreshes card
- **WHEN** a connection test persists a failed status and returns an error
- **THEN** the page SHALL refresh the connection list
- **AND** the affected card SHALL expose the persisted failed status and redacted failure summary

### Requirement: Save-as-connection validation
The create-session remote workspace section SHALL validate all required SSH profile fields before submitting when save-as-connection is enabled.

#### Scenario: Missing authentication input
- **WHEN** save-as-connection is enabled and the selected authentication mode lacks its required key path or password
- **THEN** the create action SHALL remain unavailable or present actionable validation feedback
- **AND** no session or partial SSH connection profile SHALL be created

#### Scenario: Manual remote session does not save connection
- **WHEN** save-as-connection is disabled and valid temporary remote workspace fields are provided
- **THEN** SSH profile authentication fields SHALL NOT block remote session creation
