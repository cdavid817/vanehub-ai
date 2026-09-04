# agent-task-list Specification

## Purpose
TBD - created by archiving change add-agent-task-list. Update Purpose after archive.
## Requirements
### Requirement: Session-scoped Agent task list
The system SHALL maintain at most one Agent task list per session, owned by that session and readable and writable only from it. The list SHALL be runtime-only: it SHALL NOT be persisted, SHALL NOT be restored after a desktop restart, and SHALL be discarded when its owning session ends. The list SHALL be independent of the unified Todo Board: writing it SHALL NOT create, modify, reorder, archive, or delete any board work item, and board activity SHALL NOT modify it.

#### Scenario: Each session owns its own list
- **WHEN** two sessions each write a task list
- **THEN** each session SHALL observe only its own list
- **AND** neither session's write SHALL alter the other's list

#### Scenario: List is discarded when the session ends
- **WHEN** a session that has a task list ends
- **THEN** the system SHALL discard that session's list

#### Scenario: List does not survive a desktop restart
- **WHEN** the desktop runtime starts
- **THEN** every session SHALL start with no task list

#### Scenario: Board records are untouched
- **WHEN** an Agent writes or rewrites its task list
- **THEN** the system SHALL NOT create, modify, reorder, archive, or delete any unified Todo Board work item

### Requirement: Whole-list replacement semantics
The system SHALL expose a `todo_write` tool that replaces the calling session's entire task list with the submitted list in one call and returns the resulting normalized list. The tool SHALL NOT expose per-item add, update, remove, or reorder operations, and SHALL NOT accept a session identifier from the caller. Submitting an empty list SHALL clear the list.

#### Scenario: Writing a list replaces the previous one
- **WHEN** an Agent writes a task list to a session that already has one
- **THEN** the system SHALL replace the previous list in full
- **AND** it SHALL return the resulting list rather than only an acknowledgement

#### Scenario: Item order is preserved
- **WHEN** an Agent submits items in a specific order
- **THEN** the system SHALL preserve that order in the stored and returned list

#### Scenario: Empty list clears the task list
- **WHEN** an Agent submits an empty list
- **THEN** the system SHALL clear the session's task list without error

#### Scenario: Caller cannot address another session
- **WHEN** a tool call includes a session identifier or any other scope argument
- **THEN** the system SHALL reject the call rather than writing to a session the runtime did not select

### Requirement: Task list invariants and bounds
Each item SHALL carry non-empty text and exactly one status from `pending`, `in_progress`, and `completed`. The system SHALL reject a list that exceeds its declared maximum item count, that contains an item whose text exceeds its declared maximum length, that contains an item with empty text, that contains an unrecognized status, or that contains more than one `in_progress` item. A rejected write SHALL leave the previous list unchanged.

#### Scenario: More than one item is in progress
- **WHEN** an Agent submits a list with two or more `in_progress` items
- **THEN** the system SHALL reject the write with an explicit error
- **AND** the session's previous list SHALL remain unchanged

#### Scenario: List exceeds its item bound
- **WHEN** an Agent submits more items than the declared maximum
- **THEN** the system SHALL reject the write with an explicit error rather than truncating silently

#### Scenario: Item text is empty or oversized
- **WHEN** an Agent submits an item whose text is empty, whitespace-only, or longer than the declared maximum
- **THEN** the system SHALL reject the write with an explicit error

#### Scenario: Unrecognized status
- **WHEN** an Agent submits an item whose status is not one of the three recognized values
- **THEN** the system SHALL reject the write with an explicit error

#### Scenario: A list with no in-progress item is valid
- **WHEN** an Agent submits a list whose items are all `pending`, all `completed`, or a mix of the two
- **THEN** the system SHALL accept the write

### Requirement: Task list projection into the system prompt
When a session's task list is non-empty, the system SHALL include it as a bounded, clearly labelled section of that session's generation system prompt, showing each item's text and status in list order. The section SHALL be omitted entirely when the list is empty. Because the section is part of the system prompt rather than the message history, it SHALL remain present and current after context compaction.

#### Scenario: Non-empty list reaches the provider request
- **WHEN** a generation starts for a session whose task list is non-empty
- **THEN** the outgoing system prompt SHALL contain a task-list section reflecting the current items and statuses

#### Scenario: Empty list contributes no section
- **WHEN** a generation starts for a session whose task list is empty
- **THEN** the outgoing system prompt SHALL NOT contain a task-list section

#### Scenario: List survives context compaction
- **WHEN** context compaction replaces earlier conversation turns during a generation
- **THEN** the task-list section SHALL still be present in the system prompt
- **AND** it SHALL reflect the current list rather than a summarized approximation

#### Scenario: Section reflects the most recent write
- **WHEN** an Agent rewrites its task list and a later generation starts in that session
- **THEN** the projected section SHALL reflect the most recent write

### Requirement: User-facing task workspace paths
The task board SHALL render a task workspace path without an operating-system namespace prefix that is not meaningful to the user.

#### Scenario: Windows extended path is displayed
- **WHEN** a task workspace path begins with a Windows extended-length namespace prefix
- **THEN** the board displays its ordinary user-facing path
- **AND** copying or opening the workspace uses the original valid path value

