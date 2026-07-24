# terminal-output-search Specification

## Purpose
TBD - created by archiving change add-remote-terminal-management. Update Purpose after archive.
## Requirements
### Requirement: Bounded Terminal output capture
The desktop runtime SHALL capture normalized remote Terminal and quick-command output in a dedicated SQLite user-content store without blocking UI streaming.

#### Scenario: Capture remote output
- **WHEN** an SSH PTY or exec channel emits valid UTF-8 output while capture is enabled
- **THEN** the runtime SHALL stream it to the owning UI and enqueue ordered normalized text for batched persistence

#### Scenario: Persistence is slow
- **WHEN** the bounded capture queue cannot accept additional content
- **THEN** the Terminal UI SHALL continue receiving output
- **AND** the store SHALL record a searchable capture-gap marker when persistence recovers

#### Scenario: Capture disabled
- **WHEN** output capture is disabled for the applicable context
- **THEN** output SHALL remain visible in the live Terminal and SHALL NOT create output-content rows

### Requirement: Searchable normalized output
The system SHALL index captured Terminal output with SQLite FTS5 and expose bounded paginated search through the service boundary.

#### Scenario: Search output
- **WHEN** a user submits a non-empty search query with optional session, connection, Terminal, run, or time filters
- **THEN** the service SHALL return relevance-ordered matches with bounded snippets, timestamps, source context, and a stable pagination cursor

#### Scenario: Search paths and multilingual text
- **WHEN** captured output contains paths, partial identifiers, or supported multilingual text
- **THEN** the configured index and query normalization SHALL support substring-oriented matching covered by migration tests

#### Scenario: Reject unbounded query
- **WHEN** a search request exceeds query, filter, page-size, or cursor bounds
- **THEN** the service SHALL reject or clamp it without executing an unbounded database scan

### Requirement: Output retention and capacity
The Terminal content store SHALL enforce age retention and a global capacity ceiling independently from unified log retention.

#### Scenario: Remove expired content
- **WHEN** scheduled maintenance finds output older than the configured retention window
- **THEN** it SHALL delete matching content and FTS entries in bounded transactions

#### Scenario: Enforce capacity
- **WHEN** captured content exceeds the global capacity ceiling
- **THEN** maintenance SHALL remove the oldest eligible output until the store returns within its limit

#### Scenario: Explicitly purge session output
- **WHEN** a user confirms deletion of captured output for a session
- **THEN** the system SHALL delete its content and search entries without deleting the session, templates, or unrelated runs

### Requirement: Terminal content privacy boundary
Terminal content persistence SHALL remain separate from unified diagnostic logging and SHALL never include reconstructed interactive input.

#### Scenario: Persist Terminal output
- **WHEN** Terminal output is stored in SQLite
- **THEN** lifecycle diagnostics MAY record safe ids, counts, status, and timing
- **AND** unified logs SHALL NOT copy the raw command or output content

#### Scenario: Password prompt input
- **WHEN** a remote program requests hidden or visible interactive input
- **THEN** input bytes SHALL NOT be written to command history or the output-content store by the input path

### Requirement: Web output search simulation
The Web/mock adapter SHALL provide deterministic bounded capture, search, retention, and purge behavior without writing a native SQLite database.

#### Scenario: Search Web output
- **WHEN** a Web user searches deterministic captured fixtures
- **THEN** the adapter SHALL return interface-compatible simulated results and identify simulated sources

