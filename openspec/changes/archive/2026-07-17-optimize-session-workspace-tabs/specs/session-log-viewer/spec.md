## ADDED Requirements

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

