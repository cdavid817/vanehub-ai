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
The workspace SHALL provide a scheduled-task management dialog that lets users create and manage scheduled tasks.

#### Scenario: Open scheduled task dialog
- **WHEN** the user activates the Scheduled Tasks activity entry
- **THEN** the workspace SHALL open a scheduled-task management dialog
- **AND** it SHALL NOT create a task or invoke Agent runtime behavior until the user submits a valid task

#### Scenario: Render creation fields
- **WHEN** the scheduled-task dialog is open
- **THEN** it SHALL show localized fields for task name, task content, Agent tool, frequency type, and frequency parameters
- **AND** the task name field SHALL provide a default hint such as "例如：每日整理项目进度"

#### Scenario: Render task list
- **WHEN** the scheduled-task dialog is open
- **THEN** it SHALL show created scheduled tasks with name, selected Agent, frequency summary, enabled state, next run time, and latest status

#### Scenario: Manage task state
- **WHEN** a user enables, disables, or deletes a scheduled task
- **THEN** the dialog SHALL perform the mutation through the frontend service boundary and refresh the task list

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
