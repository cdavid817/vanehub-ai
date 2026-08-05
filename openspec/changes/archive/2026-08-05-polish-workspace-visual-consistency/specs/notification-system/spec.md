## MODIFIED Requirements

### Requirement: Bounded notification lifecycle
The framework SHALL retain a bounded set of recent in-memory notifications, SHALL mark new entries unread, and SHALL manage toast visibility separately from retained history.

#### Scenario: Toast expires
- **WHEN** a notification's configured toast duration elapses
- **THEN** its toast leaves the viewport and its recent-history entry remains available in the notification center

#### Scenario: Notification volume exceeds the bound
- **WHEN** publishing a notification would exceed the configured history or visible-toast limit
- **THEN** the framework removes or hides the oldest eligible items while retaining the newest entries

#### Scenario: User manages history
- **WHEN** the user marks entries read, removes an entry, marks all entries read, or clears the center
- **THEN** notification state and unread count update consistently

#### Scenario: Toast timer survives navigation
- **WHEN** the user navigates between workspace tabs, sessions, or routes while a toast's dismiss timer is running
- **THEN** the timer SHALL keep running against its original configured duration
- **AND** the toast SHALL still leave the viewport once that duration elapses instead of persisting indefinitely
