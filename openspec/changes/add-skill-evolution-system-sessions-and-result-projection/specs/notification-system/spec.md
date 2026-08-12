## ADDED Requirements

### Requirement: Canonical evolution projection notifications
Evolution notifications SHALL be derived from the same canonical safe activity envelope used by system sessions and dashboards, with independent target delivery receipts and user threshold/digest policy.

#### Scenario: Attention event is projected
- **WHEN** an event meets notification policy
- **THEN** the notification references the same event id, safe parameters, severity, and navigation descriptor as the system timeline

#### Scenario: Event was already notified
- **WHEN** catch-up or rebuild sees its source again
- **THEN** the existing notification receipt prevents duplicate publication

### Requirement: Notification and activity read coordination
Opening a notification SHALL navigate to its activity or detail target and MAY advance the associated system-session read cursor only after the referenced item becomes visible. Dismissing a notification MUST NOT delete activity.

#### Scenario: Notification target is not yet projected
- **WHEN** the user opens a notification while timeline delivery is delayed
- **THEN** the UI opens the relevant detail or pending state without falsely marking unseen activity read

### Requirement: Evolution notification digests
The notification service SHALL support bounded per-scope digests for non-urgent evolution outcomes while security, integrity, apply failure, regression, and breaker events remain individually attention eligible according to policy.

#### Scenario: Several informational results occur
- **WHEN** digest mode is enabled
- **THEN** the system emits one bounded summary with counts and navigation rather than repetitive individual notifications

### Requirement: Projected notification actions remain non-mutating
Notifications derived from activity envelopes SHALL only navigate or adjust notification/read presentation state. They MUST NOT execute any evolution action.

#### Scenario: Automatic application notification is opened
- **WHEN** the user activates it
- **THEN** the application opens its system activity or Overlay history without reverting or approving content

