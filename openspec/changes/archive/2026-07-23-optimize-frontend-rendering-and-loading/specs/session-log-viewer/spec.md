## ADDED Requirements

### Requirement: Virtualized loaded session logs
The Logs tab SHALL virtualize loaded newest-first log entries so mounted log articles remain bounded by the viewport while preserving existing filtering, pagination, de-duplication, context display, and export behavior.

#### Scenario: Render loaded log entries
- **WHEN** one or more session log entries are loaded
- **THEN** the Logs tab SHALL mount only viewport-visible entries plus no more than ten overscan entries before and after the visible range
- **AND** each mounted entry SHALL preserve its stable log id, timestamp, level, category, message, and redacted context

#### Scenario: Scroll variable-height entries
- **WHEN** log messages or structured contexts produce different article heights
- **THEN** the virtualized list SHALL measure rendered entries
- **AND** scrolling SHALL not overlap, clip, duplicate, or reorder entries

#### Scenario: Load another log page
- **WHEN** the user activates the terminal load-more item
- **THEN** the Logs tab SHALL request the next bounded page through `agentService`
- **AND** append only entries whose ids are not already loaded

#### Scenario: Change log filters
- **WHEN** the user changes selected levels or submits a search term
- **THEN** the Logs tab SHALL clear prior pagination, reset the virtual viewport, and load the first matching page

### Requirement: Timestamp log navigation
The Logs tab SHALL let users locate the first filtered log entry whose timestamp is at or before a requested timestamp without performing unbounded retrieval.

#### Scenario: Locate within loaded entries
- **WHEN** the requested timestamp is covered by the currently loaded range
- **THEN** the Logs tab SHALL scroll the first entry at or before that timestamp into view
- **AND** move programmatic focus to the located article

#### Scenario: Locate in older paginated entries
- **WHEN** the requested timestamp is older than the loaded tail and another cursor is available
- **THEN** one locate action SHALL load and search no more than ten additional bounded pages in sequence
- **AND** it SHALL preserve active level and text filters

#### Scenario: Pause a deep timestamp search
- **WHEN** ten additional pages have been searched and the target remains older while another cursor is available
- **THEN** the Logs tab SHALL report that the target is not yet loaded
- **AND** it SHALL allow the user to continue the same search without discarding loaded entries

#### Scenario: Timestamp is outside available range
- **WHEN** the requested timestamp is newer than the newest matching entry or older than the oldest matching entry after pagination is exhausted
- **THEN** the Logs tab SHALL show a localized no-match message
- **AND** SHALL NOT focus an unrelated entry

#### Scenario: Timestamp input is invalid
- **WHEN** the user submits an empty or invalid timestamp
- **THEN** the locate action SHALL make no service request
- **AND** the Logs tab SHALL show localized validation feedback
