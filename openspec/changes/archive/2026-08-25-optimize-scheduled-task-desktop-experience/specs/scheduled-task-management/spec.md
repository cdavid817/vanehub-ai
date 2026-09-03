## MODIFIED Requirements

### Requirement: Scheduled task dialog
The workspace SHALL provide a responsive scheduled-task management dialog that lets users create and manage scheduled tasks through the frontend service boundary.

#### Scenario: Open scheduled task dialog
- **WHEN** the user activates the Scheduled Tasks activity entry
- **THEN** the workspace SHALL open a scheduled-task management dialog
- **AND** focus SHALL move into the dialog
- **AND** it SHALL NOT create a task or invoke Agent runtime behavior until the user submits a valid task

#### Scenario: Render creation fields
- **WHEN** the scheduled-task dialog is open
- **THEN** it SHALL show localized and accessibly named fields for task name, task content, stable Agent selection, frequency type, and every frequency parameter
- **AND** weekday names, interval units, hints, and validation feedback SHALL use the active application locale
- **AND** the task name field SHALL provide a default hint such as "例如：每日整理项目进度"

#### Scenario: Validate creation input
- **WHEN** required text, Agent selection, or recurrence parameters are missing or outside their supported range
- **THEN** the dialog SHALL identify the invalid field and keep creation unavailable
- **AND** it SHALL NOT submit a create request

#### Scenario: Render task list
- **WHEN** the scheduled-task dialog is open
- **THEN** it SHALL show created scheduled tasks with name, content summary, selected Agent, localized frequency summary, enabled state, next run time, and latest status
- **AND** loading, empty, failed, running, succeeded, skipped, enabled, and disabled states SHALL be visually distinguishable without relying only on color
- **AND** the list and creation controls SHALL remain usable without clipping at desktop and narrow widths

#### Scenario: Preserve tasks during refresh
- **WHEN** the dialog refreshes a previously loaded task list
- **THEN** it SHALL keep the existing tasks visible while presenting refresh progress
- **AND** a failed refresh SHALL preserve the last successfully loaded tasks and expose an actionable error

#### Scenario: Manage task state
- **WHEN** a user enables, disables, or deletes a scheduled task
- **THEN** the dialog SHALL perform the mutation through the frontend service boundary
- **AND** it SHALL show progress on the affected task and prevent conflicting duplicate mutations until the operation settles
- **AND** a successful mutation SHALL update the rendered task list from the native or Web runtime result

#### Scenario: Mutation fails
- **WHEN** a create, enable, disable, or delete mutation fails
- **THEN** the dialog SHALL retain the user's relevant context and display the error without clearing the existing task list

