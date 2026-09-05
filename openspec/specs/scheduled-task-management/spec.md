# scheduled-task-management Specification

## Purpose
Defines durable scheduled task records, recurrence configuration, runtime execution, startup catch-up behavior, frontend service boundaries, and unified logging for session-based scheduled Agent work.
## Requirements
### Requirement: Scheduled task entity contract
The system SHALL expose scheduled tasks as durable records with stable id, name, task content, selected stable Agent id, recurrence configuration, enabled state, next run timestamp, latest status, latest run timestamp, created timestamp, and updated timestamp fields.

#### Scenario: Create scheduled task record
- **WHEN** a user creates a scheduled task with a valid name, content, Agent id, and recurrence configuration
- **THEN** the system SHALL return a scheduled task record with a stable id, enabled state, next run timestamp, latest status, created timestamp, and updated timestamp

#### Scenario: Preserve stable Agent identity
- **WHEN** a scheduled task references Claude Code, Codex CLI, Gemini CLI, or OpenCode
- **THEN** the scheduled task SHALL store the selected stable Agent id rather than matching by display name

#### Scenario: Reject unsupported Agent
- **WHEN** scheduled task creation receives an unsupported Agent id
- **THEN** the system SHALL reject the request without creating a scheduled task

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

#### Scenario: Open scheduled task dialog
- **WHEN** the user activates the Scheduled Tasks entry
- **THEN** the workbench SHALL navigate to the `/runs/schedules` page rather than opening a large management dialog, per "Open scheduled tasks"
- **AND** it SHALL NOT create a task or invoke Agent runtime behavior until the user submits a valid task from the editor sheet

#### Scenario: Render creation fields
- **WHEN** the editor sheet is open for creation
- **THEN** it SHALL show localized fields for task name, task content, Agent tool, frequency type, and frequency parameters, per "Create or edit"
- **AND** the task name field SHALL provide a default hint such as "例如：每日整理项目进度"

#### Scenario: Validate creation input
- **WHEN** required text, Agent selection, or recurrence parameters are missing or outside their supported range
- **THEN** the dialog SHALL identify the invalid field and keep creation unavailable
- **AND** it SHALL NOT submit a create request

#### Scenario: Render task list
- **WHEN** the `/runs/schedules` page is open
- **THEN** it SHALL show created scheduled tasks with name, selected Agent, frequency summary, enabled state, next run time, and latest status, per "Render scheduled-task collection"

#### Scenario: Preserve tasks during refresh
- **WHEN** the page refreshes a previously loaded task list
- **THEN** it SHALL keep the existing tasks visible while presenting refresh progress
- **AND** a failed refresh SHALL preserve the last successfully loaded tasks and expose an actionable error

#### Scenario: Manage task state
- **WHEN** a user enables, disables, or deletes a scheduled task
- **THEN** the page SHALL perform the mutation through the frontend service boundary using target-local pending state and refresh the affected task without reloading the whole collection

#### Scenario: Mutation fails
- **WHEN** a create, enable, disable, or delete mutation fails
- **THEN** the page SHALL retain the user's relevant context and display the error without clearing the existing task list

### Requirement: Scheduled task recurrence configuration
The system SHALL support minute, hourly, daily, weekly, and monthly recurrence configurations using structured frequency fields.

#### Scenario: Configure minute recurrence
- **WHEN** a user selects minute recurrence
- **THEN** the system SHALL require a positive minute interval and compute the next run from that interval

#### Scenario: Configure hourly recurrence
- **WHEN** a user selects hourly recurrence
- **THEN** the system SHALL require a positive hour interval and compute the next run from that interval

#### Scenario: Configure daily recurrence
- **WHEN** a user selects daily recurrence
- **THEN** the system SHALL require a time of day and compute the next run at that time on the next eligible day

#### Scenario: Configure weekly recurrence
- **WHEN** a user selects weekly recurrence
- **THEN** the system SHALL require a weekday and time of day and compute the next run at the next eligible weekly occurrence

#### Scenario: Configure monthly recurrence
- **WHEN** a user selects monthly recurrence
- **THEN** the system SHALL require a day of month and time of day and compute the next run at the next eligible monthly occurrence

### Requirement: Scheduled task execution
The desktop runtime SHALL execute each due enabled scheduled task by creating a new session for the selected Agent and sending the task content to that session.

#### Scenario: Execute due task while app is open
- **WHEN** VaneHub AI is open and an enabled scheduled task reaches its next run time
- **THEN** the desktop runtime SHALL create a new session for the task's stable Agent id
- **AND** it SHALL submit the configured task content to that session through the session chat runtime
- **AND** it SHALL update the task's latest status, latest run timestamp, and next run timestamp

#### Scenario: Do not run disabled task
- **WHEN** a scheduled task is disabled and its next run time passes
- **THEN** the runtime SHALL NOT create a session or submit task content for that task

#### Scenario: Agent unavailable during scheduled run
- **WHEN** a due scheduled task targets an unavailable Agent runtime
- **THEN** the system SHALL preserve the task
- **AND** it SHALL update latest status to failed with a concise user-displayable reason
- **AND** it SHALL write detailed diagnostics through unified logging

### Requirement: Startup missed-run backfill
The desktop runtime SHALL backfill at most one missed run for each enabled scheduled task when the application starts.

#### Scenario: Backfill one missed run
- **WHEN** the application starts and an enabled scheduled task has one or more missed run times from while VaneHub AI was closed
- **THEN** the runtime SHALL enqueue one backfill run for that task
- **AND** it SHALL compute the task's next run timestamp from the current startup time

#### Scenario: No backfill when no run missed
- **WHEN** the application starts and an enabled scheduled task has no missed run time
- **THEN** the runtime SHALL leave the latest status unchanged
- **AND** it SHALL keep or recompute the next run timestamp without creating a session

#### Scenario: Backfill does not replay every interval
- **WHEN** the application starts after multiple recurrence intervals were missed for a task
- **THEN** the runtime SHALL create at most one catch-up run for that task

### Requirement: Scheduled task service boundary
The system SHALL keep scheduled task operations behind the frontend agent service boundary with desktop and Web adapter parity.

#### Scenario: React uses service boundary
- **WHEN** React UI code lists, creates, enables, disables, deletes, or refreshes scheduled tasks
- **THEN** it SHALL call the frontend agent service interface
- **AND** it SHALL NOT call Tauri `invoke()` directly

#### Scenario: Tauri adapter handles native scheduled task calls
- **WHEN** the desktop frontend performs a scheduled task operation
- **THEN** Tauri `invoke()` usage SHALL remain inside the Tauri-specific frontend adapter

#### Scenario: Web runtime preserves contract parity
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL expose equivalent scheduled task listing and mutation behavior without requiring SQLite or local Agent CLI access

### Requirement: Scheduled task persistence and logging
The desktop runtime SHALL persist scheduled tasks through SQLite and write scheduled task diagnostics through unified log management.

#### Scenario: Persist task across restart
- **WHEN** a scheduled task is created in the desktop runtime and the app is restarted
- **THEN** the task SHALL remain available in the scheduled-task list with its configuration and latest status

#### Scenario: Log scheduled task execution
- **WHEN** a scheduled task run starts, completes, fails, is skipped, or is backfilled
- **THEN** the runtime SHALL write redacted operation details through unified log management
- **AND** it SHALL NOT create feature-local log files

### Requirement: OnePiece scheduled-task execution
Scheduled Tasks SHALL treat the native OnePiece Agent as an eligible automation target in addition to supported CLI Agents.

#### Scenario: Create a OnePiece scheduled task
- **WHEN** the user creates a scheduled task with stable Agent id `onepiece`
- **THEN** the system SHALL accept the task when OnePiece is registered and available
- **AND** the scheduled runner SHALL start a OnePiece API interaction rather than a CLI terminal interaction

#### Scenario: Reject another non-CLI Agent
- **WHEN** a scheduled task references a non-CLI Agent other than `onepiece`
- **THEN** the system SHALL reject the task with a validation error before persistence

### Requirement: Durable scheduled-task run history
The system SHALL persist a bounded history record for each Scheduled Task execution attempt with stable identity, task identity, optional Session identity, status, timestamps, and concise redacted error information.

#### Scenario: Record scheduled execution
- **WHEN** a scheduled execution succeeds, fails, is skipped, or is backfilled
- **THEN** the system SHALL append a run-history record while preserving the Scheduled Task's existing latest-run projection

#### Scenario: List scheduled execution history
- **WHEN** a caller inspects a Scheduled Task source from the board
- **THEN** it SHALL receive recent run records ordered newest first without reading feature-local log files

### Requirement: Scheduled Task board reconciliation
Scheduled Tasks SHALL participate in unified board reconciliation without changing their recurrence or enabled semantics.

#### Scenario: Reconcile Scheduled Task
- **WHEN** an enabled or disabled Scheduled Task has no existing work-item link
- **THEN** board reconciliation SHALL create one Planned work item linked to the stable Scheduled Task id

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

