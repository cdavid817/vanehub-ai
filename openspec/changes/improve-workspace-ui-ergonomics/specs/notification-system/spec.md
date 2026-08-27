## MODIFIED Requirements

### Requirement: Accessible and theme-consistent presentation
The framework SHALL present notifications using existing visual tokens, semantic icons, and accessible controls in both futuristic and minimal themes, and SHALL anchor the toast viewport where it does not cover primary workspace controls.

#### Scenario: Semantic status presentation
- **WHEN** a notification is displayed
- **THEN** its type is identifiable through text or icon semantics in addition to color and its controls have accessible names

#### Scenario: Theme change
- **WHEN** the active theme changes between futuristic and minimal
- **THEN** toast and notification-center surfaces remain readable and visually consistent with the active application shell

#### Scenario: Narrow viewport
- **WHEN** notifications are displayed on a narrow viewport
- **THEN** toast and center content remain within the viewport without overlapping essential workspace controls

#### Scenario: Toast viewport placement
- **WHEN** one or more toasts are visible on a workspace-width viewport
- **THEN** the toast viewport SHALL be anchored to the top center of the application viewport, below the top bar
- **AND** it SHALL NOT overlap the top bar, the session sidebar, the composer send control, or the information panel tab strip
- **AND** toasts SHALL remain individually dismissible and SHALL stack without hiding the newest entry
