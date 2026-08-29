# session-log-viewer Specification

## Purpose
Defines the session-scoped unified log viewer, filtering, search, pagination, and safe desktop export behavior.
## Requirements
### Requirement: Session log list

The Logs tab SHALL display bounded, newest-first, redacted unified log entries associated with the selected session or supported seat scope, using stable keyset pagination and explicit searchable-corpus coverage.

#### Scenario: Load session logs

- **WHEN** Logs is first activated for a selected session
- **THEN** the tab SHALL request a bounded page through the frontend service and render timestamp, level, category, message, safe context fields, and available run/trace/span/operation/Agent/seat correlation
- **AND** it SHALL display whether the query corpus is complete, indexing, partial, or unavailable

#### Scenario: Load more session logs

- **WHEN** more matching entries are available and the user requests them
- **THEN** the tab SHALL fetch the next keyset page without duplicating existing entries
- **AND** newly appended records SHALL NOT shift the continuation boundary represented by the original cursor

#### Scenario: No session logs

- **WHEN** no matching entries exist and coverage is complete
- **THEN** Logs SHALL show a localized empty state

#### Scenario: No rows with incomplete coverage

- **WHEN** no matching indexed entries are returned but coverage is indexing, partial, or unavailable
- **THEN** Logs SHALL identify that the result is not a complete empty corpus
- **AND** it SHALL expose safe indexing, retry, or remediation status when available

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

The desktop runtime SHALL retrieve session log pages through the `operations`-owned redacted query index and SHALL prepare exports from the redacted unified log source without holding shared registry state during filesystem scanning. Interactive queries SHALL be bounded, keyset-paginated, query-bound, and coverage-aware.

#### Scenario: Load a session-log page

- **WHEN** the Logs tab requests a page for a selected session and filters
- **THEN** the native runtime SHALL validate session authorization, query the operations log index with a bounded keyset page, and return newest-first records plus coverage
- **AND** it SHALL NOT rescan an unbounded log directory or hold unrelated shared registry state while satisfying the interactive query

#### Scenario: New log arrives between pages

- **WHEN** a newer matching log is indexed after the first page but before the next cursor is consumed
- **THEN** the next page SHALL continue from the original keyset boundary without duplication or omission among records that existed at that boundary

#### Scenario: Cursor filters do not match

- **WHEN** a cursor created for one session, seat, time, level, text, run, trace, span, or operation filter is submitted with another filter set
- **THEN** the runtime SHALL reject it as `cursor_filter_mismatch`
- **AND** it SHALL NOT interpret it as an integer offset

#### Scenario: Prepare session-log export

- **WHEN** a user requests a desktop session-log export
- **THEN** the native runtime SHALL release shared registry state before reading filtered redacted log files or opening the destination picker
- **AND** it SHALL preserve the existing service result for success, cancellation, and failure

#### Scenario: Query index is repairing

- **WHEN** retained redacted source files are not yet fully indexed
- **THEN** the query SHALL return available indexed rows and `indexing` or `partial` coverage with bounded checkpoints
- **AND** it SHALL NOT claim the searchable corpus is complete

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

### Requirement: Structured session-log correlation filters

The Logs tab SHALL support service-backed filters for run, trace, span, operation, Agent, seat, and bounded time in addition to existing level and redacted-text search.

#### Scenario: Open Logs from a span

- **WHEN** shared workspace evidence navigation supplies run, trace, and span ids
- **THEN** Logs SHALL apply the strongest supported structured filters and show them as removable active-filter chips

#### Scenario: Filter a multi-Agent session by seat

- **WHEN** the selected session has multiple seats and the user chooses one seat while Logs is active
- **THEN** the log query SHALL include that stable seat id
- **AND** entries with absent or different seat correlation SHALL not be presented as matching that seat

#### Scenario: Correlation field is unavailable

- **WHEN** a log record was emitted outside an execution context or before a correlation field was captured
- **THEN** the service SHALL leave that field absent
- **AND** the UI SHALL not fabricate a run, trace, span, operation, Agent, or seat association

### Requirement: Live session-log tail

The Logs tab SHALL support a bounded live-tail mode using post-redaction, post-index-commit notices while preserving stable page queries as the recovery source.

#### Scenario: Follow newest matching logs

- **WHEN** Follow is enabled, the user is at the newest edge, and a new indexed record matches the current filters
- **THEN** the row SHALL appear without a full page reload
- **AND** the viewport MAY remain at the newest edge

#### Scenario: User scrolls away from newest edge

- **WHEN** the user scrolls away from the newest records or pauses Follow
- **THEN** new arrivals SHALL NOT force the viewport to jump
- **AND** the UI SHALL show a bounded count or indicator with an action to jump to the newest edge

#### Scenario: Live notice cannot be evaluated locally

- **WHEN** a safe identifier-only notice lacks fields needed to decide the active text or structured filters
- **THEN** the frontend SHALL invalidate or refresh the first page rather than inserting an unverified match

#### Scenario: Live notices contain a gap

- **WHEN** a bounded subscriber queue reports dropped notices
- **THEN** the Logs tab SHALL refresh from the indexed query source
- **AND** it SHALL not assume the locally accumulated live rows are complete

#### Scenario: A live row is rendered

- **WHEN** the Logs tab inserts a row announced by a live notice
- **THEN** it SHALL read the authoritative row from the indexed query by record id
- **AND** the notice payload SHALL NOT be rendered as the row's content

#### Scenario: Subscription races the first page

- **WHEN** the Logs tab begins live tailing
- **THEN** it SHALL register its listener before reading the resume watermark
- **AND** notices that arrive in that window SHALL be reconciled against the watermark rather than dropped

### Requirement: Non-destructive log refresh and pagination failure

Logs SHALL keep previously loaded entries visible when a later refresh, live update, or load-more operation fails.

#### Scenario: Initial load fails

- **WHEN** no log page has loaded and the initial request fails
- **THEN** Logs MAY show a blocking localized error with Retry

#### Scenario: Load more fails

- **WHEN** one or more pages are already visible and a later page request fails
- **THEN** the visible entries SHALL remain rendered
- **AND** an inline page error and Retry action SHALL appear at the continuation boundary

#### Scenario: Refresh fails

- **WHEN** loaded rows are visible and a refresh request fails
- **THEN** the rows SHALL remain visible with stale/error status
- **AND** the tab SHALL not replace them with a blank full-panel error

### Requirement: Session-log query coverage presentation

The log service and Logs tab SHALL expose and present complete, indexing, partial, and unavailable query coverage plus retained/queryable time boundaries when known.

#### Scenario: Complete retained corpus

- **WHEN** all retained redacted source records for the selected scope are indexed and no known gap applies
- **THEN** coverage SHALL be `complete` with the oldest and newest available boundaries when known

#### Scenario: Source records expired or were dropped

- **WHEN** retention, source-file deletion, queue overflow, parse rejection, or repair failure creates a known gap
- **THEN** coverage SHALL be `partial` with safe reason codes and bounded counts/boundaries when known

#### Scenario: Query service is unavailable

- **WHEN** the index cannot answer safely
- **THEN** Logs SHALL preserve any cached rows, mark current data unavailable or stale, and expose Retry or remediation
- **AND** it SHALL not silently fall back to an unbounded filesystem scan

#### Scenario: A search reaches its candidate bound

- **WHEN** a text search examines its maximum bounded candidate set without exhausting the matching scope
- **THEN** the result SHALL be reported as `partial` and `truncated`
- **AND** it SHALL NOT be presented as a complete result that found no match

