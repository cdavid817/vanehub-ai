## ADDED Requirements

### Requirement: Project-derived session grouping metadata
The system SHALL expose enough workspace metadata on listed session records for consumers to group sessions by project without querying SQLite or the filesystem from React components.

#### Scenario: Local project session grouping metadata
- **WHEN** sessions are listed and a session has worktree, project, or folder metadata
- **THEN** the returned session record SHALL include the existing worktree path, project path, and folder fields needed to derive an owning project group
- **AND** React components SHALL group from service-backed session records rather than direct native or database reads

#### Scenario: Session without project metadata
- **WHEN** sessions are listed and a session has no worktree, project, folder, or remote workspace metadata
- **THEN** the returned session record SHALL remain valid
- **AND** consumers SHALL be able to place it in a localized ungrouped project bucket

#### Scenario: Preserve list ordering inside project groups
- **WHEN** sessions are rendered in project groups
- **THEN** sessions within each group SHALL preserve the stable session listing order provided by the service
