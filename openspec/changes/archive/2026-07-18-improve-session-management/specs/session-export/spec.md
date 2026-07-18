## ADDED Requirements

### Requirement: Session export formats
The system SHALL export a selected session in JSON and Markdown formats.

#### Scenario: Export session as JSON
- **WHEN** a user exports a session as JSON
- **THEN** the exported file SHALL include a version marker, session metadata, messages, thinking content, tool-use blocks, token usage, statuses, errors, file references, and export timestamp

#### Scenario: Export session as Markdown
- **WHEN** a user exports a session as Markdown
- **THEN** the exported file SHALL include readable session metadata and chronological messages with thinking content, tool-use details, statuses, errors, token usage, and file references preserved in text form

### Requirement: Desktop export destination
The desktop runtime SHALL save exported sessions to a user-selected directory through the service boundary.

#### Scenario: Export to selected directory
- **WHEN** a desktop user chooses a format and destination directory
- **THEN** Rust SHALL write the export file into that directory and return the saved path through the frontend service

#### Scenario: Export failure
- **WHEN** the destination is unavailable or the file cannot be written
- **THEN** the system SHALL show a concise localized failure and write detailed diagnostics through unified logging

### Requirement: Web export parity
The Web/mock runtime SHALL provide export behavior without claiming native filesystem side effects.

#### Scenario: Export in Web mode
- **WHEN** a Web/mock user exports a session
- **THEN** the adapter SHALL return deterministic simulated export metadata or browser-safe content and SHALL identify native directory saving as simulated or unavailable
