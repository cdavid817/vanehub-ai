## ADDED Requirements

### Requirement: File explorer directory error visibility
The Files tab SHALL surface directory-load errors in the file-tree section where the user action occurred, and SHALL NOT mark a directory as expanded when its contents failed to load.

#### Scenario: Directory expand fails
- **WHEN** the user clicks a directory and the listing service rejects the request
- **THEN** the file-tree section SHALL display a localized error notice following the existing partial-results pattern
- **AND** the directory SHALL remain visually collapsed (ChevronRight) rather than appearing empty-and-expanded

### Requirement: Git changes selection stability
The Changes tab SHALL auto-select the first status entry only on initial data load and SHALL preserve the user's manual selection across status-data refetches for the same session.

#### Scenario: Initial status load
- **WHEN** git status data arrives for the first time in a session
- **THEN** the first entry in the status list SHALL be auto-selected and its diff loaded

#### Scenario: Status data refetches
- **WHEN** git status data refetches for the same session while the user has selected a non-first entry
- **THEN** the selected entry SHALL remain unchanged

### Requirement: Git diff truncation notice
The Changes tab SHALL indicate when a loaded diff has been truncated by the backend.

#### Scenario: Diff is truncated
- **WHEN** a diff result has its `truncated` flag set
- **THEN** the diff panel SHALL display a localized partial-results notice above the diff content
