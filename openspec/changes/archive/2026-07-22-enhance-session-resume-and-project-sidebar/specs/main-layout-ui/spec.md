## ADDED Requirements

### Requirement: Resizable session sidebar
The workspace shell SHALL let users resize the expanded session sidebar horizontally within bounded minimum and maximum widths.

#### Scenario: Drag sidebar resize handle
- **WHEN** the session sidebar is expanded and the user drags its resize handle horizontally
- **THEN** the sidebar width SHALL update within bounded limits
- **AND** the main content SHALL resize without overlapping the activity bar, information panel, or status bar

#### Scenario: Persist sidebar width preference
- **WHEN** the user changes the session sidebar width
- **THEN** the workspace SHALL remember the width preference for later workspace renders in the same browser or desktop WebView profile

#### Scenario: Collapse preserves width preference
- **WHEN** the user collapses and re-expands the session sidebar after resizing it
- **THEN** the sidebar SHALL restore the last bounded expanded width
- **AND** hidden sidebar controls SHALL remain unreachable while collapsed

### Requirement: Project-grouped session sidebar
The workspace session sidebar SHALL provide a project grouping presentation that groups sessions by their owning worktree, project, folder, or remote workspace metadata.

#### Scenario: Render project groups
- **WHEN** project grouping is selected
- **THEN** the sidebar SHALL render sessions under project groups derived from service-backed session metadata
- **AND** each project group SHALL show a concise label, a session count, and an expand/collapse control

#### Scenario: Toggle project group expansion
- **WHEN** the user toggles a project group
- **THEN** the sidebar SHALL expand or collapse that project's session cards without changing the active session

#### Scenario: Ungrouped project bucket
- **WHEN** visible sessions have no project, folder, worktree, or remote workspace metadata
- **THEN** the sidebar SHALL render those sessions in a localized ungrouped project bucket

#### Scenario: Preserve filtering and archived behavior
- **WHEN** search, Agent filtering, archived view, pinned sessions, or batch-management mode is active
- **THEN** project grouping SHALL apply to the currently visible session source without bypassing existing selection and context actions
