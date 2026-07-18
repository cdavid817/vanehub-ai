## ADDED Requirements

### Requirement: Remote workspace local-tab availability
Local filesystem-backed session workspace tabs SHALL avoid reading local paths for remote workspace sessions.

#### Scenario: Open local file-backed tab for remote workspace
- **WHEN** Files, Documents, Changes, or Shell is opened for a remote workspace session before remote execution support exists
- **THEN** the service SHALL return an unavailable workspace context or reject process creation with a concise unsupported message

#### Scenario: Keep chat available for remote workspace
- **WHEN** a remote workspace session is selected
- **THEN** the Chat tab SHALL remain available and the session metadata SHALL identify the remote workspace target
