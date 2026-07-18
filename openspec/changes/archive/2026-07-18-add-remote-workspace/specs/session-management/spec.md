## ADDED Requirements

### Requirement: Remote workspace session metadata
The system SHALL expose optional remote workspace metadata on durable session records.

#### Scenario: Create session with remote workspace metadata
- **WHEN** a session is created with a remote workspace request
- **THEN** the system SHALL return a session record with remote workspace host, user, path, display name, and effective folder set to a stable remote URI

#### Scenario: Search by remote workspace metadata
- **WHEN** a user searches historical sessions by remote host, user, path, display name, or remote URI
- **THEN** matching sessions SHALL be returned without requiring React to inspect remote state

### Requirement: Remote workspace creation input
The system SHALL allow session creation input to choose either a local project/worktree target or a remote workspace target.

#### Scenario: Reject incomplete remote workspace
- **WHEN** session creation receives a remote workspace request without host or path
- **THEN** the system SHALL reject the request without creating a session

#### Scenario: Reject mixed workspace targets
- **WHEN** session creation receives a remote workspace request and a Git worktree request
- **THEN** the system SHALL reject the request without executing Git commands

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL accept the same remote workspace input and return equivalent mock session metadata
