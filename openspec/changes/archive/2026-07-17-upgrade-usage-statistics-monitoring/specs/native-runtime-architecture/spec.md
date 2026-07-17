## MODIFIED Requirements

### Requirement: Native usage statistics query
The native runtime SHALL expose a declared read-only Tauri command that aggregates normalized SQLite usage records without exposing direct database access to the frontend.

#### Scenario: Aggregate desktop usage statistics
- **WHEN** the Tauri adapter requests usage statistics for a supported time range
- **THEN** the native runtime SHALL return separated reported-token and estimated-character totals, coverage, counted sessions and responses, local-calendar daily trend points, and per-Agent rows
- **AND** it SHALL key Agent rows by stable Agent id rather than matching display names

#### Scenario: Reject unsupported usage range
- **WHEN** the frontend requests an unsupported usage statistics time range
- **THEN** the native runtime SHALL reject the request with a structured user-displayable error

#### Scenario: Keep usage query bounded
- **WHEN** the native runtime handles the usage statistics command
- **THEN** it SHALL perform indexed bounded read-only aggregate queries
- **AND** it SHALL NOT spawn external commands, scan the filesystem, access the network, or load prompt and response bodies for aggregation

#### Scenario: Use desktop-local calendar semantics
- **WHEN** the native runtime filters or groups a bounded usage range
- **THEN** it SHALL derive range boundaries and daily bucket dates from the desktop user's local calendar rather than UTC midnight

## ADDED Requirements

### Requirement: Native normalized usage persistence
The native runtime SHALL persist versioned normalized usage records in SQLite through the session runtime and database layer.

#### Scenario: Enforce one record per response
- **WHEN** the native runtime writes usage for an assistant message
- **THEN** SQLite SHALL enforce at most one usage record for that message
- **AND** session or message deletion SHALL remove the owned usage record through the ownership relationship

#### Scenario: Enforce accounting invariants
- **WHEN** a usage record is inserted or updated
- **THEN** token and character counts SHALL be non-negative
- **AND** reported accounting SHALL use token units while estimated accounting SHALL use character units

#### Scenario: Protect usage-record privacy
- **WHEN** usage accounting is persisted
- **THEN** the usage record SHALL NOT contain prompt text, response text, raw CLI events, credentials, or secret values

#### Scenario: Index monitoring dimensions
- **WHEN** the usage-record migration completes
- **THEN** occurrence-time and stable-Agent-id query dimensions SHALL have indexes suitable for bounded trend and Agent aggregation
