# scheduled-task-management Specification Delta

## ADDED Requirements

### Requirement: Scheduled task update and duplication
The scheduled-task service and UI SHALL support editing and duplicating a task with stable identity, version-aware mutation, validation, and Web/Tauri contract parity.

#### Scenario: Edit task
- **WHEN** the user submits valid changed name, content, Agent, recurrence, timezone, workspace, or enabled configuration supported by the model
- **THEN** the service SHALL update the same stable task using a current version witness and return canonical next-run state

#### Scenario: Edit loses a race
- **WHEN** the task changed after the editor loaded
- **THEN** the service SHALL reject or reconcile the stale version
- **AND** the UI SHALL keep the draft and present canonical differences

#### Scenario: Duplicate task
- **WHEN** the user duplicates an existing task
- **THEN** the UI SHALL open a new disabled draft with copied permitted configuration, a new identity only after create, and a name requiring review
- **AND** run history SHALL not be copied

#### Scenario: Adapter parity
- **WHEN** update or duplicate is used in Web or Tauri mode
- **THEN** both adapters SHALL expose the same contract shape
- **AND** Web simulation SHALL not claim native persistence

### Requirement: Scheduled task run-now action
A permitted scheduled task SHALL expose an explicit Run now action that creates one asynchronous execution through the owning scheduler or Agent service without changing the recurrence schedule.

#### Scenario: Run now
- **WHEN** the user confirms immediate execution of an eligible enabled or disabled task according to scheduler policy
- **THEN** the service SHALL return a stable operation or Run reference before completion
- **AND** the task's configured recurrence SHALL remain unchanged

#### Scenario: Duplicate request
- **WHEN** Run now is already pending for the same UI operation
- **THEN** the action SHALL prevent accidental duplicate submission while allowing inspection of existing history

#### Scenario: Agent or workspace unavailable
- **WHEN** preflight rejects immediate execution
- **THEN** no Session or Run SHALL be partially created and the UI SHALL show a safe actionable reason

### Requirement: Scheduled task run history UI
The scheduled-task page SHALL expose the existing durable run history as a bounded paginated timeline or table with status, trigger, timestamps, optional Session or Run, and safe failure classification.

#### Scenario: List history
- **WHEN** a task has execution attempts
- **THEN** records SHALL be ordered newest first and distinguish scheduled, manual, and startup catch-up triggers when known

#### Scenario: Open execution
- **WHEN** a history record references a Session or canonical Run
- **THEN** the page SHALL provide a validated EvidenceLink and safe return context

#### Scenario: History is empty
- **WHEN** a task has never run
- **THEN** the detail SHALL show a localized no-history state rather than omitting the section

#### Scenario: Load more
- **WHEN** history exceeds the initial page
- **THEN** the page SHALL request a bounded next page without reloading the task collection

### Requirement: Scheduled recurrence timezone and occurrence preview
Scheduled-task creation, editing, list, and detail SHALL display an explicit timezone and a bounded preview of future occurrences computed with the same recurrence semantics used for execution.

#### Scenario: Configure recurrence
- **WHEN** the user changes frequency, interval, weekday, day of month, time, or timezone
- **THEN** the editor SHALL show the next five eligible occurrences or a validation error before save

#### Scenario: View a saved task
- **WHEN** the task detail renders
- **THEN** the configured timezone and next occurrence SHALL be explicit
- **AND** user-visible timestamps SHALL be formatted for the active locale without losing timezone meaning

#### Scenario: Encounter daylight-saving transition
- **WHEN** a preview crosses an offset change
- **THEN** the preview and executor SHALL follow the documented timezone policy and expose the resulting local times
- **AND** the UI SHALL not silently treat local time as UTC

#### Scenario: Preview service unavailable
- **WHEN** the runtime cannot compute a trustworthy preview
- **THEN** save SHALL follow existing validation policy and the UI SHALL mark preview unavailable rather than inventing dates

### Requirement: Scheduled runtime capability disclosure
The scheduled-task page SHALL clearly disclose the current execution model, including application-open execution and at-most-one startup catch-up, without presenting the limitation as a transient error.

#### Scenario: Render capability notice
- **WHEN** the Schedules page or editor renders
- **THEN** it SHALL explain that tasks execute while VaneHub AI is running and how startup catch-up behaves

#### Scenario: Review before enabling
- **WHEN** the user enables or creates an enabled schedule
- **THEN** the final review SHALL include the execution-model notice and next occurrence

#### Scenario: Runtime model changes later
- **WHEN** a future capability provides background daemon execution
- **THEN** the UI SHALL derive the notice from a service capability contract rather than retaining hard-coded obsolete text

### Requirement: Localized scheduled recurrence labels
All scheduled-task weekday, frequency, interval, trigger, timezone, occurrence, history, validation, action, and capability text SHALL use synchronized locale resources and locale-aware formatters.

#### Scenario: Render weekdays
- **WHEN** weekly recurrence choices or summaries render
- **THEN** weekday labels SHALL come from locale-aware resources or formatting and SHALL not use a hard-coded English array

#### Scenario: Render recurrence summary
- **WHEN** a minute, hourly, daily, weekly, or monthly task appears
- **THEN** the summary SHALL remain concise and unambiguous in the active locale

#### Scenario: Render stable identifiers
- **WHEN** a task, Agent, operation, Session, or Run id is shown in diagnostic detail
- **THEN** the identifier MAY remain literal while surrounding labels and actions use the active locale

### Requirement: Scheduled task action hierarchy
Scheduled-task rows and detail SHALL expose Enable or Disable, Run now, Edit, Duplicate, and Delete with state-aware hierarchy, local pending feedback, and guarded destructive behavior.

#### Scenario: Render row actions
- **WHEN** a task row is visible
- **THEN** its enabled state and most common permitted action SHALL be direct
- **AND** secondary actions SHALL use an accessible action menu

#### Scenario: Delete task
- **WHEN** the user chooses Delete
- **THEN** the UI SHALL confirm the effect on future execution and retained history according to domain policy
- **AND** Delete SHALL not sit beside the enable control as an equal accidental target

#### Scenario: Mutation pending
- **WHEN** one task action is pending
- **THEN** only conflicting actions for that task SHALL be disabled
- **AND** the rest of the page SHALL remain operable

## MODIFIED Requirements

### Requirement: Scheduled task dialog
The workbench SHALL replace the full scheduled-task management dialog with a first-class `/runs/schedules` page and MAY retain a small quick-create dialog only as a shortcut that navigates to or submits through the same service contract.

#### Scenario: Open scheduled tasks
- **WHEN** the user selects Schedules under Runs or opens a legacy Scheduled Tasks entry
- **THEN** the workbench SHALL navigate to the scheduled-task management page
- **AND** it SHALL not require a large modal to browse or manage existing tasks

#### Scenario: Render scheduled-task collection
- **WHEN** the page opens
- **THEN** it SHALL show a bounded searchable and filterable task list with name, Agent, localized recurrence, enabled state, timezone, next run, latest status, and attention state

#### Scenario: Open task detail
- **WHEN** the user selects a scheduled task
- **THEN** the route SHALL expose configuration, next-occurrence preview, capability notice, latest run, and bounded run history

#### Scenario: Create or edit
- **WHEN** the user starts creation or editing
- **THEN** an accessible editor sheet SHALL open with fields and validation adjacent to the task context
- **AND** the list query and selected task SHALL remain recoverable

#### Scenario: Use compact width
- **WHEN** list and detail cannot fit together
- **THEN** the page SHALL use list-then-detail and full-height editor-sheet flows with clear Back and Close actions
