# settings-skill-management-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Service-backed Skills settings page
The Skills settings page SHALL render as a service-backed Skill management surface rather than a static demo data page.

#### Scenario: Load Skills settings data
- **WHEN** a user opens the Skills settings page
- **THEN** the page SHALL load Skills, registered Agents, Agent mount paths, Skill statistics, and drift status through the frontend service boundary

#### Scenario: No static demo data
- **WHEN** the Skills settings page renders
- **THEN** the page SHALL NOT use hard-coded demo Skill arrays as the source of displayed Skill data

### Requirement: Skills page module composition
The Skills settings page SHALL be composed from seven reusable child components: `SkillStatsCards`, `SkillAgentMountPathsPanel`, `SkillScopeTabs`, `SkillFilterToolbar`, `SkillCardList`, `SkillDialogs`, and `SkillDriftBanner`.

#### Scenario: Render Skill management modules
- **WHEN** the Skills settings page has loaded data
- **THEN** it SHALL show statistics, Agent mount paths, scope controls, filters, Skill cards, dialogs, drift status, and bottom summary behavior through the composed modules

### Requirement: Skill statistics and summary
The Skills settings page SHALL display core Skill metrics and a bottom summary for the active scope and filters.

#### Scenario: Display Skill statistics
- **WHEN** the page renders loaded Skill data
- **THEN** it SHALL show counts for all Skills, enabled Skills, and mounted Skills

#### Scenario: Display filtered summary
- **WHEN** a user changes scope, category, search query, enabled state, or Agent binding
- **THEN** the bottom summary SHALL reflect the current visible Skill set and active scope

### Requirement: Agent mount path panel
The Skills settings page SHALL show all registered Agents with editable Skill mount paths.

#### Scenario: Display Agent mount paths
- **WHEN** registered Agents are loaded
- **THEN** the page SHALL display each Agent with its current Skill mount path as a code-style label

#### Scenario: Edit Agent mount path
- **WHEN** a user changes an Agent mount path
- **THEN** the page SHALL submit the change through the frontend service boundary and display the migration result returned by the service

### Requirement: Skill scope selection
The Skills settings page SHALL support `global` and `workspace` scope selection.

#### Scenario: Switch to global scope
- **WHEN** a user selects the global scope tab
- **THEN** the page SHALL load global Skills and global drift status

#### Scenario: Select workspace directory
- **WHEN** a user selects the workspace scope
- **THEN** the page SHALL provide a directory picker for choosing the local project directory

#### Scenario: Workspace scope load
- **WHEN** a workspace directory is selected
- **THEN** the page SHALL load Skills and drift status for that workspace directory only

### Requirement: Skill filtering and search
The Skills settings page SHALL allow users to filter Skills by category and search by keyword.

#### Scenario: Category filter
- **WHEN** a user selects a Skill category
- **THEN** the Skill card list SHALL show only Skills in that category

#### Scenario: Keyword search
- **WHEN** a user enters a search query
- **THEN** the Skill card list SHALL match Skills by id, name, description, category, triggers, or source label

### Requirement: Skill card controls
Each Skill card SHALL provide enablement, Agent binding, source labeling, preview, edit, and delete controls.

#### Scenario: Toggle Skill enabled state
- **WHEN** a user toggles a Skill enabled state
- **THEN** the page SHALL submit the change through the frontend service boundary and refresh the affected Skill and drift state

#### Scenario: Toggle Agent binding
- **WHEN** a user changes Agent binding checkboxes on a Skill card
- **THEN** the page SHALL submit the binding set through the frontend service boundary and refresh the affected Skill and drift state

#### Scenario: Source badge
- **WHEN** a Skill card renders
- **THEN** it SHALL display whether the Skill source is built-in, user-created, or imported

### Requirement: Skill dialogs
The Skills settings page SHALL provide dialogs for `SKILL.md` preview, Skill creation, Skill editing, external Skill import, and built-in Skill restore.

#### Scenario: Preview SKILL.md
- **WHEN** a user opens Skill preview
- **THEN** the dialog SHALL display the current `SKILL.md` source content loaded through the frontend service boundary

#### Scenario: Create Skill
- **WHEN** a user submits a valid create Skill form
- **THEN** the page SHALL create a Skill with immutable id and valid `SKILL.md` frontmatter through the frontend service boundary

#### Scenario: Edit Skill
- **WHEN** a user edits a Skill
- **THEN** the form SHALL prevent changing the Skill id and SHALL submit editable metadata and body through the frontend service boundary

#### Scenario: Import external Skill
- **WHEN** a user imports an external Skill directory
- **THEN** the page SHALL call the frontend service boundary to copy it into the selected scope and refresh the Skill list

#### Scenario: Restore built-in Skill
- **WHEN** a user restores a deleted built-in Skill
- **THEN** the page SHALL call the frontend service boundary and refresh built-in Skill availability

### Requirement: Skill drift banner
The Skills settings page SHALL display a drift banner when Skill registry, source files, or mount paths are inconsistent.

#### Scenario: Display drift issues
- **WHEN** drift detection reports one or more issues
- **THEN** the page SHALL show a banner with the issue count and a path to review or synchronize the issues

#### Scenario: Synchronize drift
- **WHEN** a user activates one-click drift synchronization
- **THEN** the page SHALL call the frontend service boundary and display the synchronization report, including backup and overwrite results
