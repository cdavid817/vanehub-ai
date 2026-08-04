## MODIFIED Requirements

### Requirement: Service-backed Skills settings page
The Skills settings page SHALL render from one service-backed overview per active scope and SHALL expose explicit loading, error, stale-conflict, and partial-operation states rather than interpreting absent data as an empty healthy result.

#### Scenario: Load Skills settings data
- **WHEN** a user opens the Skills settings page
- **THEN** the page SHALL load Skills, compatible CLI/API Agents, Agent mount paths, both binding types, Skill statistics, drift status, and restore candidates through the frontend service boundary

#### Scenario: Load failure
- **WHEN** the overview request fails or drift is not yet available
- **THEN** the page SHALL display a loading or error state and SHALL NOT display an empty list or an in-sync success state as if it were confirmed data

#### Scenario: No static demo data
- **WHEN** the Skills settings page renders
- **THEN** it SHALL NOT use hard-coded demo Skill arrays as the source of displayed Skill data

### Requirement: Skill card controls
Each Skill card SHALL provide enablement, CLI Agent mount binding, API Agent prompt binding, source labeling, preview, edit, and guarded delete controls, with the two Agent target types displayed separately.

#### Scenario: Toggle Skill enabled state
- **WHEN** a user toggles a Skill enabled state
- **THEN** the page SHALL submit the change through the frontend service boundary, prevent a duplicate pending mutation, and refresh the affected overview state

#### Scenario: Toggle Agent binding
- **WHEN** a user changes a CLI Agent binding checkbox
- **THEN** the page SHALL submit a granular bind or unbind operation and SHALL NOT lose another completed checkbox change

#### Scenario: Toggle API Agent binding
- **WHEN** a user changes an API Agent binding checkbox
- **THEN** the page SHALL submit the non-mount binding change without creating or editing a filesystem mount path

#### Scenario: Agent type separation
- **WHEN** Skill cards and the mount-path panel render
- **THEN** API-only Agents SHALL NOT appear in CLI mount controls and CLI-only Agents SHALL NOT appear in API prompt-binding controls

#### Scenario: Source badge
- **WHEN** a Skill card renders
- **THEN** it SHALL display whether the Skill source is built-in, user-created, or imported

### Requirement: Skill dialogs
The Skills settings page SHALL provide dialogs for `SKILL.md` preview, Skill creation, conflict-aware Skill editing, bounded external Skill import, and restore of currently deleted built-in Skills.

#### Scenario: Preview SKILL.md
- **WHEN** a user opens Skill preview
- **THEN** the dialog SHALL display the current `SKILL.md` source content loaded through the frontend service boundary

#### Scenario: Create Skill
- **WHEN** a user submits a valid create Skill form
- **THEN** the page SHALL create a Skill with immutable id and valid `SKILL.md` frontmatter through the frontend service boundary

#### Scenario: Edit Skill
- **WHEN** a user opens an existing Skill for editing
- **THEN** the form SHALL load its current metadata and body, prevent changing the id, and submit the previewed content hash for conflict detection

#### Scenario: Stale edit conflict
- **WHEN** the submitted content hash no longer matches the live Skill document
- **THEN** the dialog SHALL remain open, explain that the Skill changed, and offer a reload without overwriting the newer document

#### Scenario: Import external Skill
- **WHEN** a user imports an external Skill directory
- **THEN** the page SHALL call the frontend service boundary, display validation or limit failures, and refresh the Skill overview only after success

#### Scenario: Restore built-in Skill
- **WHEN** a user opens built-in restore
- **THEN** the dialog SHALL list only currently deleted built-in Skill ids returned by the service

#### Scenario: Guard destructive deletion
- **WHEN** a user requests deletion of a user-created or imported Skill
- **THEN** the page SHALL require confirmation before removing its managed source directory
