## MODIFIED Requirements

### Requirement: Indexed desktop message search
Desktop historical-session search MUST use an SQLite full-text index for ordinary persisted message substring queries and MUST return session records plus bounded match context without per-result database loads. Search MUST bound the work it performs, not only the number of rows it returns: it MUST NOT rank or sort every matching message in the database in order to produce a limited result page.

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

#### Scenario: Short query resolves match context through the session index
- **WHEN** the desktop repository runs the short-query compatibility path
- **THEN** it SHALL resolve each candidate session's newest matching message through the session/created-at message index
- **AND** it SHALL NOT materialize and rank the full set of matching messages across all sessions

#### Scenario: Short query returns the same results as the ranking form
- **WHEN** the same two-character query is served by the bounded compatibility path
- **THEN** it SHALL return the same sessions, in the same order, as ranking every matching message would
- **AND** each returned session's match context SHALL be its newest matching message
