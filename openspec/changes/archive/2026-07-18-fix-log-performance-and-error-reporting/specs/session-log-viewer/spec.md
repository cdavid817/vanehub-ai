## ADDED Requirements

### Requirement: Bounded native session-log retrieval
The desktop runtime SHALL retrieve session log pages and export candidates without holding the shared registry state during filesystem scanning and SHALL bound interactive log reads.

#### Scenario: Load a session-log page
- **WHEN** the Logs tab requests a page for a selected session
- **THEN** the native runtime SHALL resolve session authorization and the active log directory before releasing the shared registry state
- **AND** it SHALL read newest matching log data within a fixed retrieval bound
- **AND** it SHALL return a newest-first page or a truncated result without blocking unrelated registry operations on file I/O

#### Scenario: Prepare session-log export
- **WHEN** a user requests a desktop session-log export
- **THEN** the native runtime SHALL release the shared registry state before reading filtered log files or opening the destination picker
- **AND** it SHALL preserve the existing service result for success, cancellation, and failure
