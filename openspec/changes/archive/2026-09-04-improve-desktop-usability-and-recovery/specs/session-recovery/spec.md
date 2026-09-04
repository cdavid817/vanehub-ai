## ADDED Requirements

### Requirement: User initiated recoverable session reconnect
The system SHALL expose an explicit recovery action for a failed or disconnected session when the runtime can safely attempt to resume it, including from the session context menu.

#### Scenario: Recover a failed session
- **WHEN** a user selects recovery for a recoverable failed session
- **THEN** the system attempts reconnection through the session service boundary
- **AND** reports the resulting running, failed, or unavailable state without discarding session history

#### Scenario: Recovery is unavailable
- **WHEN** a failed session cannot safely be resumed
- **THEN** the recovery action explains why it is unavailable
- **AND** the user retains access to session history and diagnostics
