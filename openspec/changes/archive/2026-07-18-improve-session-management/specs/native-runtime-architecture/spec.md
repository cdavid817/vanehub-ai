## ADDED Requirements

### Requirement: Native session maintenance jobs
The desktop runtime SHALL run session maintenance jobs in Rust after database and unified logging initialization.

#### Scenario: Start maintenance jobs
- **WHEN** the desktop runtime initializes successfully
- **THEN** it SHALL run startup recovery and automatic archival checks without blocking the main window UI

#### Scenario: Hourly automatic archival schedule
- **WHEN** automatic archival is enabled
- **THEN** Rust SHALL schedule a recurring check approximately once per hour while the application remains running

### Requirement: Native session search and export
The desktop runtime SHALL own persisted session search queries and filesystem export writes.

#### Scenario: Search persisted history
- **WHEN** the frontend searches historical sessions in desktop mode
- **THEN** Rust SHALL query SQLite for session metadata and message content and return bounded results

#### Scenario: Write export file
- **WHEN** the frontend requests desktop session export with a selected destination directory
- **THEN** Rust SHALL serialize the requested session and write the JSON or Markdown file to that directory

### Requirement: Native file reference validation
The desktop runtime SHALL validate chat file references against the owning session root before including file content in an Agent prompt.

#### Scenario: Validate referenced file
- **WHEN** a message includes file references
- **THEN** Rust SHALL confirm each file resolves inside the session root and satisfies size and text-content limits before reading it

#### Scenario: Log unsafe reference rejection
- **WHEN** a file reference is rejected for safety or availability reasons
- **THEN** Rust SHALL return a concise user-displayable error and write redacted diagnostics through unified logging
