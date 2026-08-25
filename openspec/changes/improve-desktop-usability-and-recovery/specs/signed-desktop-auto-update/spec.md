## ADDED Requirements

### Requirement: Recoverable manual update check
The About update surface SHALL provide an explicit retry action after a check failure and SHALL retain a readable safe error until the user retries or leaves the surface.

#### Scenario: Update check fails
- **WHEN** a manual update check reaches a failed terminal state
- **THEN** the About surface presents the safe failure reason and a retry action
- **AND** a retry starts a new asynchronous check without requiring an application restart
