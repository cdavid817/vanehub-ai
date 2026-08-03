## ADDED Requirements

### Requirement: Global Skill Agent navigation
The Skills settings page SHALL organize the global Skill library through dynamic stable-Agent navigation with All Skills, compatible CLI Agent, compatible API Agent, and Unassigned views.

#### Scenario: Render Agent navigation
- **WHEN** the global Skill overview contains compatible Agents
- **THEN** the page SHALL render each Agent from its stable id and display name without hard-coded provider branches
- **AND** known Agents SHALL use the registered visual identity and brand icon

#### Scenario: Select CLI Agent
- **WHEN** a user selects a CLI-capable Agent
- **THEN** the page SHALL show separately labeled Assigned and Available global Skills for that Agent
- **AND** SHALL describe the assignment as a CLI mount binding

#### Scenario: Select API Agent
- **WHEN** a user selects an API Agent
- **THEN** the page SHALL show separately labeled Assigned and Available global Skills for that Agent
- **AND** SHALL describe the assignment as API prompt injection rather than a filesystem mount

#### Scenario: Show unassigned global Skills
- **WHEN** a global Skill has no CLI or API Agent binding
- **THEN** the Unassigned view SHALL include that Skill regardless of its enabled state

## MODIFIED Requirements

### Requirement: Service-backed Skills settings page
The Skills settings page SHALL render the global Skill library from one service-backed global overview and SHALL expose explicit loading, retryable error, stale-conflict, empty, pending, and partial-operation states rather than interpreting absent data as an empty healthy result.

#### Scenario: Load global Skills settings data
- **WHEN** a user opens the Skills settings page
- **THEN** the page SHALL load global Skills, compatible CLI/API Agents, global Agent mount paths, both binding types, global Skill statistics, global drift status, and restore candidates through the frontend service boundary
- **AND** SHALL use `{ scope: "global", workspacePath: null }` for its overview and mutation context

#### Scenario: Exclude project selection from Settings
- **WHEN** the Skills settings page renders
- **THEN** it SHALL NOT ask the user to select or enter a workspace path
- **AND** SHALL NOT load or mutate project Skills from the Settings surface

#### Scenario: Load failure
- **WHEN** the global overview request fails or global drift is not yet available
- **THEN** the page SHALL display a loading or retryable error state
- **AND** SHALL NOT display an empty list or in-sync state as confirmed data

#### Scenario: Targeted mutation state
- **WHEN** an operation targets a specific global Skill, binding, Agent mount path, or dialog
- **THEN** its pending or failed state SHALL be presented at the affected control or interaction surface
- **AND** unrelated global Skill controls SHALL remain available unless the service operation requires broader serialization

#### Scenario: No static demo data
- **WHEN** the Skills settings page renders
- **THEN** it SHALL NOT use hard-coded demo Skill arrays as the source of displayed Skill data

### Requirement: Skills page module composition
The Skills settings page SHALL use focused child components for dynamic Agent navigation, global inventory filtering, assigned/available lists, global Skill dialogs, global drift status, and selected-CLI mount configuration while keeping service query and mutation orchestration in the page container.

#### Scenario: Render global Skill management modules
- **WHEN** the Skills settings page has loaded data
- **THEN** it SHALL render a responsive two-column Agent-navigation and inventory layout consistent with CLI Parameter Management
- **AND** each child component SHALL receive service-backed data and callbacks instead of invoking runtime APIs directly

#### Scenario: Preserve maintainable component boundaries
- **WHEN** Agent navigation, Skill lifecycle dialogs, or inventory rendering require additional UI surfaces
- **THEN** the implementation SHALL keep those concerns in focused components without causing the page container or a child component to exceed the project file-size limit

### Requirement: Skill statistics and summary
The Skills settings page SHALL display a compact global summary and SHALL display counts relevant to the selected All Skills, Agent, or Unassigned view.

#### Scenario: Display global Skill summary
- **WHEN** the page renders the All Skills view
- **THEN** it SHALL show total, enabled, CLI-bound, API-bound, and unassigned global Skill counts without requiring multiple full-size statistic cards

#### Scenario: Display selected Agent summary
- **WHEN** a user selects a compatible Agent
- **THEN** the inventory SHALL show assigned, active or paused, and available global Skill counts for that stable Agent id

#### Scenario: Display filtered result count
- **WHEN** a user changes selected Agent, assigned/available view, category, source, status, sort order, or keyword search
- **THEN** the inventory toolbar SHALL reflect the current visible global Skill count

### Requirement: Agent mount path panel
The Skills settings page SHALL provide the selected CLI-capable Agent's editable global Skill mount path inside a default-collapsed advanced disclosure and SHALL not show filesystem mount controls for API-only Agents.

#### Scenario: Keep mount paths secondary
- **WHEN** a CLI Agent view loads without a failed mount-path migration
- **THEN** its mount-path editor SHALL remain collapsed by default
- **AND** assigned and available global Skills SHALL remain reachable before expanding advanced settings

#### Scenario: Display selected CLI mount path
- **WHEN** a user expands advanced settings for a selected CLI Agent
- **THEN** the page SHALL display that Agent's current Skill mount path as a code-style editable value

#### Scenario: Exclude API mount configuration
- **WHEN** an API Agent view is active
- **THEN** the page SHALL NOT display a filesystem mount-path editor

#### Scenario: Edit Agent mount path
- **WHEN** a user changes the selected CLI Agent's mount path
- **THEN** the page SHALL submit the change through the frontend service boundary and display the migration result returned by the service

#### Scenario: Failed migration remains visible
- **WHEN** a mount-path migration reports one or more failures
- **THEN** the page SHALL expose the failure summary without requiring the user to discover it inside a collapsed disclosure

### Requirement: Skill filtering and search
The Skills settings page SHALL use the settings top-bar query as its single keyword search and SHALL allow users to filter and sort the current global inventory view without changing persisted Skill data.

#### Scenario: Keyword search
- **WHEN** a user enters a query in the settings top-bar Skill search
- **THEN** the current global inventory SHALL match Skills by id, name, description, category, triggers, or localized source label
- **AND** the page SHALL NOT present a second competing keyword input

#### Scenario: Combine inventory filters
- **WHEN** a user selects category, source, or enabled-state filters within an All, selected-Agent, or Unassigned view
- **THEN** the global Skill inventory SHALL apply all active filters together
- **AND** SHALL display the resulting count

#### Scenario: Sort Skill inventory
- **WHEN** a user selects a supported sort order
- **THEN** the current inventory SHALL reorder the filtered global Skills deterministically without changing persisted Skill data

#### Scenario: Clear Skill filters
- **WHEN** one or more inventory filters are active and the user activates clear filters
- **THEN** the page SHALL restore the default filters and sort order while preserving the selected Agent or inventory view

### Requirement: Skill card controls
Each global Skill inventory row SHALL provide bounded metadata, source and version labeling, global enablement state, preview, edit, guarded delete, and assignment controls appropriate to the active All Skills, CLI Agent, API Agent, or Unassigned view.

#### Scenario: Compact inventory remains bounded by Agent count
- **WHEN** the global overview contains many compatible Agents
- **THEN** a Skill row SHALL NOT render the complete Agent checkbox matrix
- **AND** Agent-specific assignment SHALL be performed in the selected Agent view

#### Scenario: Toggle global Skill enabled state
- **WHEN** a user toggles a global Skill enabled state from All Skills
- **THEN** the page SHALL explain that enablement applies to the global Skill across its bindings
- **AND** SHALL submit the change through the frontend service boundary, prevent a duplicate pending mutation, and refresh the global overview
- **AND** SHALL preserve every existing CLI and API Agent assignment without assigning the Skill to any additional Agent

#### Scenario: Keep global enablement read-only in selected Agent views
- **WHEN** a user views Assigned or Available Skills for a selected CLI or API Agent
- **THEN** each row SHALL present global enabled, paused, or unavailable status without rendering a mutable global enablement control
- **AND** a paused assigned Skill MAY provide navigation to All Skills where global enablement is managed

#### Scenario: Assign global Skill to CLI Agent
- **WHEN** a user assigns or removes a global Skill in a selected CLI Agent view
- **THEN** the page SHALL submit a granular CLI bind or unbind operation using the selected stable Agent id
- **AND** SHALL NOT change global Skill enablement or any other Agent assignment
- **AND** SHALL NOT lose another completed binding change

#### Scenario: Assign global Skill to API Agent
- **WHEN** a user assigns or removes a global Skill in a selected API Agent view
- **THEN** the page SHALL submit the non-mount API bind or unbind operation using the selected stable Agent id
- **AND** SHALL NOT change global Skill enablement or any other Agent assignment
- **AND** SHALL NOT create or edit a filesystem mount path

#### Scenario: Explain configured and active CLI bindings
- **WHEN** a global Skill is disabled while retaining a CLI Agent binding
- **THEN** the selected CLI view SHALL identify it as assigned but paused rather than currently mounted

#### Scenario: Source and version labels
- **WHEN** a global Skill row renders
- **THEN** it SHALL display whether the Skill source is built-in, user-created, or imported
- **AND** SHALL display its version without allowing long metadata to resize the inventory layout

### Requirement: Skill dialogs
The Skills settings page SHALL provide accessible application dialogs for readable global `SKILL.md` preview, global Skill creation, conflict-aware editing, bounded external import, confirmed deletion, and restore of currently deleted built-in global Skills.

#### Scenario: Preview global SKILL.md
- **WHEN** a user opens global Skill preview
- **THEN** the dialog SHALL load the current global `SKILL.md` content through the frontend service boundary
- **AND** SHALL provide a readable Markdown presentation and access to the source content

#### Scenario: Create global Skill
- **WHEN** a user submits a valid create Skill form from Settings
- **THEN** the page SHALL create a global Skill with immutable id and valid `SKILL.md` frontmatter through the frontend service boundary

#### Scenario: Edit global Skill
- **WHEN** a user opens an existing global Skill for editing
- **THEN** the form SHALL load its current metadata and body, prevent changing the id, and submit the loaded content hash for conflict detection
- **AND** SHALL provide Edit and Preview modes for the Markdown body

#### Scenario: Stale edit conflict
- **WHEN** the submitted content hash no longer matches the live global Skill document
- **THEN** the dialog SHALL remain open, explain that the Skill changed, and offer a reload without overwriting the newer document

#### Scenario: Import global Skill
- **WHEN** a user imports an external Skill directory from Settings
- **THEN** the page SHALL create it in global scope, display validation or limit failures, and refresh the global overview only after success

#### Scenario: Restore built-in global Skill
- **WHEN** a user opens built-in restore
- **THEN** the dialog SHALL list only currently deleted built-in global Skill ids returned by the service

#### Scenario: Guard destructive deletion
- **WHEN** a user requests deletion of a global Skill
- **THEN** the page SHALL use a localized application confirmation dialog before removing its managed source directory
- **AND** SHALL NOT rely on the browser-native confirmation prompt

#### Scenario: Dialog accessibility
- **WHEN** a global Skill dialog opens or closes
- **THEN** it SHALL expose a translated accessible name, contain keyboard focus while open, support keyboard dismissal when safe, and restore focus to the triggering control

### Requirement: Skill drift banner
The Skills settings page SHALL present healthy global drift status as a compact indicator and SHALL display a prominent actionable banner when global Skill registry, source files, or CLI mount paths are inconsistent or a synchronization result needs review.

#### Scenario: Display healthy global drift status
- **WHEN** global drift detection completes with no issues and no synchronization result requires review
- **THEN** the page SHALL show a compact in-sync indicator without inserting a full-width success banner above the global inventory

#### Scenario: Display global drift issues
- **WHEN** global drift detection reports one or more issues
- **THEN** the page SHALL show a prominent banner with the issue count, a bounded issue summary, and a path to synchronize the issues

#### Scenario: Synchronize global drift
- **WHEN** a user activates one-click global drift synchronization
- **THEN** the page SHALL call the frontend service boundary and display the synchronization report, including backup, overwrite, restored, and failed results

#### Scenario: Preserve actionable synchronization result
- **WHEN** global synchronization completes with failures or backup/overwrite activity
- **THEN** the result SHALL remain reviewable until the user explicitly dismisses it or leaves the page

## REMOVED Requirements

### Requirement: Skill scope selection
**Reason**: Settings now has a single application-level responsibility: global Skill administration. Project scope is derived from the active session and managed in Information Panel → Skill, so a Settings scope switcher and manual project-directory picker are ambiguous and redundant.

**Migration**: Existing global and workspace Skill records remain unchanged. Global operations continue in Settings; project Skill presentation and management move to the active session information panel using `worktreePath` with `projectPath` fallback.
