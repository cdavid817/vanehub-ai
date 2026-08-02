## ADDED Requirements

### Requirement: Bounded historical search scheduling
The session search UI MUST suppress trivial or superseded requests and MUST preserve the service boundary in both desktop and Web/mock runtimes.

#### Scenario: Type a historical search query
- **WHEN** a user changes the search text repeatedly within 250 milliseconds
- **THEN** the UI SHALL submit only the latest trimmed query after the quiet period
- **AND** it SHALL NOT submit a query shorter than two characters

#### Scenario: Search through either runtime
- **WHEN** a debounced query is submitted in desktop or Web/mock mode
- **THEN** React SHALL use the shared frontend session service interface
- **AND** the result count SHALL remain bounded by the requested service limit

### Requirement: Indexed desktop message search
Desktop historical-session search MUST use an SQLite full-text index for ordinary persisted message substring queries and MUST return session records plus bounded match context without per-result database loads.

#### Scenario: Search existing indexed messages
- **WHEN** a query of at least three characters matches persisted message content
- **THEN** SQLite SHALL use the maintained message-content FTS index
- **AND** the repository SHALL return the owning sessions and match context from one result query

#### Scenario: Keep the message index synchronized
- **WHEN** a persisted message is inserted, its content is updated, or it is deleted
- **THEN** the SQLite FTS index SHALL reflect that change transactionally

#### Scenario: Upgrade an existing database
- **WHEN** the database migration runs with existing persisted messages
- **THEN** it SHALL backfill those messages into the FTS index
- **AND** historical search SHALL continue to include archived sessions

#### Scenario: Search a two-character query
- **WHEN** the desktop repository receives a two-character query that cannot use the trigram index
- **THEN** it SHALL use a bounded compatibility query
- **AND** it SHALL return the same service result shape
