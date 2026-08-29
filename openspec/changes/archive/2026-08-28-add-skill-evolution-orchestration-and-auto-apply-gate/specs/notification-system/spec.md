## ADDED Requirements

### Requirement: Evolution orchestration notifications
The notification system SHALL publish sanitized, deduplicated notifications for partial or failed runs requiring attention, successful automatic application, probation regression, and circuit-breaker opening or recovery, with navigation to the relevant Skill Evolution view.

#### Scenario: Automatic application succeeds
- **WHEN** learned guidance commits automatically
- **THEN** one notification identifies the safe Skill, application id, probation end, and navigation target without including guidance or diff content

#### Scenario: Routine run completes
- **WHEN** a run completes without mutation or attention-required outcome
- **THEN** the system does not produce repetitive success notifications unless policy explicitly requests them

### Requirement: Orchestration notification actions are non-mutating
Notification actions SHALL only navigate and MUST NOT enable policy, close a breaker, cancel a run, approve Curator work, or revert an Overlay.

#### Scenario: User opens breaker notification
- **WHEN** the notification is activated
- **THEN** the application opens breaker health details without acknowledging it

