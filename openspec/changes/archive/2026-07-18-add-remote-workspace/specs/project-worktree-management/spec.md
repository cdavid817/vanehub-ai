## ADDED Requirements

### Requirement: Remote workspace history
The system SHALL maintain a history of remote workspaces used during session creation.

#### Scenario: Record remote workspace
- **WHEN** a session is created with a remote workspace target
- **THEN** the system SHALL persist host, optional user, path, display name, URI, and last opened timestamp

#### Scenario: List remote workspaces
- **WHEN** the create-session dialog requests known remote workspaces
- **THEN** the system SHALL return previously used remote workspaces ordered by most recently opened first

#### Scenario: Preserve local project history
- **WHEN** local project history is requested
- **THEN** remote workspace entries SHALL NOT be mixed into the local project history list
