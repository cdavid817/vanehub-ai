## ADDED Requirements

### Requirement: Session messages belong to their session
The system SHALL associate persisted chat messages with their owning session record.

#### Scenario: List messages for selected session
- **WHEN** messages are listed for a session id
- **THEN** only messages owned by that session SHALL be returned

#### Scenario: Delete session removes messages
- **WHEN** a session with persisted messages is deleted
- **THEN** persisted messages for that session SHALL be deleted through the session ownership relationship

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** session-owned mock messages SHALL follow the same ownership contract without requiring SQLite
