## ADDED Requirements

### Requirement: Native usage statistics query
The native runtime SHALL expose a declared read-only Tauri command for usage statistics aggregation from SQLite chat message storage.

#### Scenario: Aggregate desktop usage statistics
- **WHEN** the Tauri adapter requests usage statistics for a supported time range
- **THEN** the native runtime SHALL aggregate `token_input` and `token_output` from persisted messages in SQLite and return the summary without exposing direct database access to the frontend

#### Scenario: Reject unsupported usage range
- **WHEN** the frontend requests an unsupported usage statistics time range
- **THEN** the native runtime SHALL reject the request with a structured user-displayable error

#### Scenario: Keep usage query bounded
- **WHEN** the native runtime handles the usage statistics command
- **THEN** it SHALL perform a bounded read-only aggregate query and SHALL NOT spawn external commands, scan the filesystem, or perform network work
