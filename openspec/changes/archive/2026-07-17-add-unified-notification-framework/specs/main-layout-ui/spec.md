## ADDED Requirements

### Requirement: Interactive top-bar notification center
The main layout SHALL connect the existing top-bar notification control to the unified notification state and SHALL provide a compact recent-notification popover.

#### Scenario: Unread notifications exist
- **WHEN** at least one retained notification is unread
- **THEN** the Bell control exposes a visible unread indicator and an accessible unread count

#### Scenario: User opens the notification center
- **WHEN** the user activates the Bell control
- **THEN** an anchored popover presents recent notifications, read state, timestamps, and available management controls without navigating away from the workspace

#### Scenario: Notification center is empty
- **WHEN** the user opens the center with no retained notifications
- **THEN** the popover presents a localized empty state and does not show unavailable management actions

#### Scenario: User dismisses the popover
- **WHEN** the user presses Escape, activates the Bell control again, or clicks outside the popover
- **THEN** the notification center closes and focus behavior remains usable
