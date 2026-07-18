# session-log-viewer Specification

## Purpose
Defines the session-scoped unified log viewer, filtering, search, pagination, and safe desktop export behavior.

## Requirements
### Requirement: Session log list
The Logs tab SHALL display bounded, newest-first unified log entries associated with the selected session.

#### Scenario: Load session logs
- **WHEN** Logs is first activated for a selected session
- **THEN** the tab SHALL request a bounded page through the frontend service and render timestamp, level, category, message, and safe context fields

#### Scenario: Load more session logs
- **WHEN** more matching entries are available and the user requests them
- **THEN** the tab SHALL fetch the next page without duplicating existing entries

#### Scenario: No session logs
- **WHEN** no matching entries exist
- **THEN** Logs SHALL show a localized empty state

### Requirement: Log filtering and search
The Logs tab SHALL support error, warn, info, and debug level selection plus case-insensitive text search.

#### Scenario: Filter levels
- **WHEN** the user changes selected log levels
- **THEN** the tab SHALL request or display only entries matching the selected levels

#### Scenario: Search logs
- **WHEN** the user submits non-empty search text
- **THEN** the tab SHALL match redacted category, message, and safe context text without searching unredacted source data

#### Scenario: Clear filters
- **WHEN** the user clears search and restores all levels
- **THEN** Logs SHALL return to the selected session's unfiltered bounded log view

### Requirement: Safe log export
The Logs tab SHALL expose desktop export through the service boundary and SHALL communicate cancellation, success, and unavailability with localized messages.

#### Scenario: Complete desktop export
- **WHEN** the user confirms a destination for the current session and filters
- **THEN** Logs SHALL report the destination returned by the native export result without reading or writing the file directly

#### Scenario: Cancel desktop export
- **WHEN** the user cancels the destination picker
- **THEN** Logs SHALL remain unchanged and SHALL NOT show a failure notification

#### Scenario: Request Web export
- **WHEN** export is unavailable in Web/mock mode
- **THEN** the control SHALL be disabled or return a localized unavailable explanation without claiming a download

### Requirement: Bounded native session-log retrieval
The desktop runtime SHALL retrieve session log pages and export candidates without holding the shared registry state during filesystem scanning and SHALL bound interactive log reads.

#### Scenario: Load a session-log page
- **WHEN** the Logs tab requests a page for a selected session
- **THEN** the native runtime SHALL resolve session authorization and the active log directory before releasing the shared registry state
- **AND** it SHALL read newest matching log data within a fixed retrieval bound
- **AND** it SHALL return a newest-first page or a truncated result without blocking unrelated registry operations on file I/O

#### Scenario: Prepare session-log export
- **WHEN** a user requests a desktop session-log export
- **THEN** the native runtime SHALL release the shared registry state before reading filtered log files or opening the destination picker
- **AND** it SHALL preserve the existing service result for success, cancellation, and failure
