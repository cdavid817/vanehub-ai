## ADDED Requirements

### Requirement: Dual Skill scopes
The system SHALL manage Skills in isolated `global` and `workspace` scopes.

#### Scenario: Global Skills use home scope
- **WHEN** a user lists Skills for the `global` scope
- **THEN** the system SHALL return only global Skills stored under the fixed user-home VaneHub Skill directory

#### Scenario: Workspace Skills use project boundary
- **WHEN** a user lists Skills for the `workspace` scope with a workspace directory
- **THEN** the system SHALL return only Skills stored under that workspace directory's VaneHub Skill directory

#### Scenario: Same Skill id in different scopes
- **WHEN** the same Skill id exists in both global and workspace scopes
- **THEN** the system SHALL manage their enabled state, source path, Agent bindings, drift state, and deletion independently

### Requirement: Standard SKILL.md metadata
The system SHALL use `SKILL.md` as the required definition file for every Skill and SHALL parse a fixed frontmatter schema containing `id`, `name`, `description`, `category`, `version`, and optional `triggers`.

#### Scenario: Valid Skill metadata
- **WHEN** a Skill directory contains a `SKILL.md` with valid required frontmatter
- **THEN** the system SHALL parse the metadata and expose it in Skill list, preview, create, edit, import, and drift responses

#### Scenario: Missing SKILL.md
- **WHEN** a Skill registry record points to a directory that does not contain `SKILL.md`
- **THEN** the system SHALL report drift for that Skill instead of treating it as healthy

#### Scenario: Immutable Skill id
- **WHEN** a user edits an existing Skill
- **THEN** the system SHALL reject attempts to change the Skill `id`

### Requirement: Built-in Skill seeds
The system SHALL provide six built-in Skills: `tdd-discipline`, `code-review`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`.

#### Scenario: Idempotent built-in initialization
- **WHEN** built-in Skill initialization runs more than once
- **THEN** the system SHALL NOT create duplicate registry records or duplicate Skill directories

#### Scenario: Deleted built-in is not auto-restored
- **WHEN** a user deletes a built-in Skill and built-in initialization runs later
- **THEN** the system SHALL keep the Skill deleted until the user explicitly restores it

#### Scenario: Restore built-in Skill
- **WHEN** a user restores a deleted built-in Skill
- **THEN** the system SHALL recreate the standard `SKILL.md`, registry record, and source directory for that built-in Skill

### Requirement: Agent mount path management
The system SHALL use registered Agent ids as Skill mount carriers and SHALL store one editable mount path per Agent.

#### Scenario: Default Agent mount paths
- **WHEN** the system returns mount paths for registered Agents
- **THEN** it SHALL include defaults for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode` when those Agents are registered

#### Scenario: Update Agent mount path
- **WHEN** a user changes an Agent mount path
- **THEN** the system SHALL persist the new path for that Agent and immediately migrate existing managed Skill links for that Agent

#### Scenario: Migration report
- **WHEN** an Agent mount path migration completes
- **THEN** the system SHALL return a report containing migrated, removed, overwritten, backed up, and failed Skill entries

### Requirement: Skill Agent bindings and link mounts
The system SHALL bind Skills to zero or more registered Agents and SHALL mount bound enabled Skills into each Agent's configured mount path by symlink or directory link.

#### Scenario: Bind Skill to Agent
- **WHEN** a user binds an enabled Skill to an Agent
- **THEN** the system SHALL create a managed link from the Agent mount path to the Skill source directory

#### Scenario: Unbind Skill from Agent
- **WHEN** a user removes an Agent binding from a Skill
- **THEN** the system SHALL remove that Skill's managed link from the Agent mount path without deleting the Skill source directory

#### Scenario: Disable Skill
- **WHEN** a user disables a Skill
- **THEN** the system SHALL remove managed links for that Skill while preserving its source directory and metadata record

### Requirement: External Skill import
The system SHALL import external Skills by copying the external Skill directory into the selected scope's managed Skill source directory.

#### Scenario: Import valid external Skill
- **WHEN** a user imports an external directory containing a valid `SKILL.md`
- **THEN** the system SHALL copy it into the selected scope, create a registry record, and make it available for Agent binding

#### Scenario: Import invalid external Skill
- **WHEN** a user imports an external directory without valid required `SKILL.md` metadata
- **THEN** the system SHALL reject the import and SHALL NOT create a registry record

### Requirement: Skill drift detection
The system SHALL detect drift between SQLite registry records, source `SKILL.md` files, and Agent mount paths.

#### Scenario: Source file changed
- **WHEN** a Skill source `SKILL.md` content hash differs from the registry hash
- **THEN** the system SHALL report metadata or content drift for that Skill

#### Scenario: Registry missing for source Skill
- **WHEN** a managed Skill source directory exists with `SKILL.md` but no registry record exists for the selected scope
- **THEN** the system SHALL report an unregistered Skill drift issue

#### Scenario: Missing mount
- **WHEN** an enabled Skill is bound to an Agent but no managed link exists in that Agent's mount path
- **THEN** the system SHALL report a missing mount drift issue

#### Scenario: Conflicting mount target
- **WHEN** a file, directory, or foreign link occupies a bound Skill target path
- **THEN** the system SHALL report a conflict drift issue for that Skill and Agent

### Requirement: Skill drift synchronization
The system SHALL provide synchronization that repairs drift and uses backup-and-overwrite for conflicting mount targets.

#### Scenario: Sync missing mount
- **WHEN** synchronization runs for a Skill with a missing bound mount
- **THEN** the system SHALL recreate the managed link in the Agent mount path

#### Scenario: Sync conflict with backup
- **WHEN** synchronization encounters a conflicting mount target
- **THEN** the system SHALL move the conflicting target to a backup path before creating the managed link

#### Scenario: Sync report
- **WHEN** synchronization finishes
- **THEN** the system SHALL return a report containing mounted, unmounted, overwritten, backed up, restored, and failed entries

### Requirement: Service boundary for Skill operations
The system SHALL expose all Skill operations through the frontend service boundary and SHALL implement equivalent Tauri and Web/mock adapter methods.

#### Scenario: Desktop Skill operation
- **WHEN** the Tauri runtime performs a Skill operation from the settings page
- **THEN** the React component SHALL call `AgentService`, the Tauri frontend adapter SHALL call a Tauri command, and the Rust layer SHALL perform SQLite or filesystem work

#### Scenario: Web Skill operation
- **WHEN** the Web runtime performs a Skill operation from the settings page
- **THEN** the Web adapter SHALL return deterministic mock Skill data without requiring local filesystem access
