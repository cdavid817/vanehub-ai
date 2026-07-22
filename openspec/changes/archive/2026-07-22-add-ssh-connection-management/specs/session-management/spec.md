## ADDED Requirements

### Requirement: Remote session creation from SSH connection
The system SHALL allow remote session creation to derive remote workspace input from a selected SSH connection profile while preserving session-local remote metadata.

#### Scenario: Select SSH connection for remote session
- **WHEN** a user creates a remote session by selecting an SSH connection profile
- **THEN** the created session SHALL store a remote workspace snapshot derived from the profile host, port, user, effective path, display name, and stable URI
- **AND** the session SHALL remain readable without loading the source SSH connection profile

#### Scenario: Override SSH connection default path
- **WHEN** a user selects an SSH connection profile and changes the remote path before creating the session
- **THEN** the created session SHALL use the overridden path in its remote workspace snapshot
- **AND** the SSH connection default path SHALL remain unchanged

#### Scenario: Save temporary remote input as connection
- **WHEN** a user manually enters remote host, port, user, path, and authentication details in the create-session remote section and chooses to save them as a connection
- **THEN** the system SHALL create the SSH connection profile through the service boundary before or during session creation
- **AND** it SHALL still create the session from a remote workspace snapshot

#### Scenario: Preserve manual temporary remote session
- **WHEN** a user manually enters remote host, port, user, and path without saving them as a connection
- **THEN** the system SHALL create a remote session snapshot without creating a durable SSH connection profile
