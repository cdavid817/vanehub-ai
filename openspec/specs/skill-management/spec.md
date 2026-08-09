# skill-management Specification

## Purpose
TBD - created by archiving change add-skill-management-settings. Update Purpose after archive.
## Requirements
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
The system SHALL provide six built-in Skills: `tdd-discipline`, `code-review`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`. Built-in initialization SHALL reconcile the registry with what already exists on disk rather than assuming an empty filesystem, and SHALL report a per-Skill outcome so one Skill's failure cannot leave the others unregistered.

#### Scenario: Idempotent built-in initialization
- **WHEN** built-in Skill initialization runs more than once
- **THEN** the system SHALL NOT create duplicate registry records or duplicate Skill directories

#### Scenario: Deleted built-in is not auto-restored
- **WHEN** a user deletes a built-in Skill and built-in initialization runs later
- **THEN** the system SHALL keep the Skill deleted until the user explicitly restores it

#### Scenario: Restore built-in Skill
- **WHEN** a user restores a deleted built-in Skill
- **THEN** the system SHALL recreate the standard `SKILL.md`, registry record, and source directory for that built-in Skill

#### Scenario: Adopt an existing source that has no registry record
- **WHEN** built-in initialization finds a built-in Skill's source directory already present while no registry record exists for it, and the Skill is not marked deleted
- **THEN** the system SHALL register the existing source instead of failing
- **AND** it SHALL leave the on-disk `SKILL.md` unmodified
- **AND** the resulting record SHALL describe the content that is actually on disk

#### Scenario: Adopted content that diverges from the shipped definition is reported, not overwritten
- **WHEN** an adopted source's content differs from the shipped built-in definition
- **THEN** the system SHALL report that difference, naming the affected Skills, as part of the initialization diagnostic
- **AND** it SHALL NOT silently replace the user's file

#### Scenario: One unusable built-in does not block the rest
- **WHEN** initialization cannot register one built-in Skill
- **THEN** the system SHALL still register every other built-in Skill it can
- **AND** it SHALL name which Skills succeeded and which failed, with a reason for each failure

#### Scenario: An already-present built-in is not an error
- **WHEN** initialization encounters a built-in Skill whose source is already present
- **THEN** the system SHALL NOT emit an `error`-level log for that condition
- **AND** any diagnostic it does emit SHALL be attributed to the operation that produced it

### Requirement: Agent mount path management
The system SHALL use registered CLI-capable Agent ids as Skill mount carriers, SHALL store one editable mount path per CLI-capable Agent, and SHALL reject mount paths that overlap the VaneHub-managed `.vanehub` namespace or any Skill source directory.

#### Scenario: Default Agent mount paths
- **WHEN** the system returns mount paths for registered Agents
- **THEN** it SHALL include defaults for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli` when those CLI Agents are registered
- **AND** SHALL exclude API-only Agents

#### Scenario: Update Agent mount path
- **WHEN** a user changes a CLI Agent mount path to a valid disjoint relative path
- **THEN** the system SHALL persist the new path for that Agent and immediately migrate existing managed Skill links for that Agent

#### Scenario: Reject managed namespace overlap
- **WHEN** a requested mount path is `.vanehub`, is below `.vanehub`, or otherwise overlaps a resolved Skill source path
- **THEN** the system SHALL reject it before moving, backing up, or linking any filesystem entry

#### Scenario: Migration report
- **WHEN** an Agent mount path migration completes
- **THEN** the system SHALL return a report containing migrated, removed, overwritten, backed up, and failed Skill entries

### Requirement: Skill Agent bindings and link mounts
The system SHALL bind Skills to zero or more registered CLI-capable Agents and SHALL mount bound enabled Skills into each CLI Agent's configured mount path by symlink or directory link.

#### Scenario: Bind Skill to CLI Agent
- **WHEN** a user binds an enabled Skill to a registered CLI-capable Agent
- **THEN** the system SHALL create a managed link from the Agent mount path to the Skill source directory

#### Scenario: Reject CLI mount binding for API Agent
- **WHEN** a caller attempts to create a CLI mount binding for an API-only Agent
- **THEN** the system SHALL reject the binding without changing SQLite or the filesystem

#### Scenario: Unbind Skill from Agent
- **WHEN** a user removes a CLI Agent binding from a Skill
- **THEN** the system SHALL remove that Skill's managed link from the Agent mount path without deleting the Skill source directory

#### Scenario: Disable Skill
- **WHEN** a user disables a Skill
- **THEN** the system SHALL remove managed links for that Skill while preserving its source directory, metadata record, and API bindings

### Requirement: Canonical workspace Skill identity
The system SHALL canonicalize an existing workspace directory before using it as a Workspace Skill storage key and filesystem boundary.

#### Scenario: Equivalent workspace aliases
- **WHEN** two path spellings resolve to the same physical workspace directory
- **THEN** the system SHALL address one Workspace Skill scope rather than creating independent records for the aliases

#### Scenario: Conflicting legacy aliases
- **WHEN** existing database rows for equivalent workspace aliases cannot be merged without ambiguity
- **THEN** the system SHALL report a reconciliation conflict and SHALL NOT overwrite either Skill source

### Requirement: Skill source lifecycle safety
The system SHALL preserve non-`SKILL.md` files during ordinary edits and SHALL reject implicit replacement of an existing unregistered source directory.

#### Scenario: Edit imported Skill with assets
- **WHEN** a user edits the metadata or body of an imported Skill containing scripts, templates, or resource files
- **THEN** the system SHALL atomically replace only `SKILL.md` and SHALL preserve every other file

#### Scenario: Create collides with unregistered source
- **WHEN** create, import, built-in seed, or restore resolves to an existing source directory that is not the exact permitted lifecycle state
- **THEN** the system SHALL report a conflict and SHALL NOT replace or delete the directory

#### Scenario: Restore eligibility
- **WHEN** a caller restores a built-in Skill
- **THEN** the system SHALL require that the Skill is absent and has a matching deleted-built-in tombstone

### Requirement: Bounded external Skill import
The system SHALL reject imports whose source overlaps the managed destination or exceeds configured document, file-count, depth, or aggregate-size limits.

#### Scenario: Import source contains destination
- **WHEN** the selected import source contains or is contained by the resolved managed destination
- **THEN** the system SHALL reject the import before recursive copying begins

#### Scenario: Import exceeds a limit
- **WHEN** an import exceeds 512 files, depth 16, 16 MiB aggregate bytes, or a 256 KiB `SKILL.md`
- **THEN** the system SHALL abort and roll back the partial managed target

### Requirement: Serializable and conflict-aware Skill mutation
The native runtime SHALL serialize Skill filesystem/database mutations and SHALL reject an edit based on a stale content hash.

#### Scenario: Concurrent binding changes
- **WHEN** independent CLI Agent bindings are changed in rapid succession
- **THEN** each granular bind or unbind SHALL be applied without losing another completed binding change

#### Scenario: Stale document edit
- **WHEN** `SKILL.md` changed after the user loaded the edit form
- **THEN** the update SHALL fail with a conflict and SHALL preserve the newer document

### Requirement: Intentional built-in deletion is not drift
The system SHALL keep deleted built-in tombstones separate from filesystem/registry drift and SHALL expose them only as explicit restore candidates.

#### Scenario: Detect drift after built-in deletion
- **WHEN** a user intentionally deleted a built-in Skill
- **THEN** drift detection SHALL NOT report that tombstone as an inconsistency

#### Scenario: Synchronize drift after built-in deletion
- **WHEN** synchronization runs while a built-in Skill remains intentionally deleted
- **THEN** it SHALL NOT restore the Skill or clear its tombstone

### Requirement: Batch Skill management overview
The system SHALL return the selected scope's Skills, CLI/API bindings, statistics, mount paths, compatible Agents, drift state, and restore candidates through one service-boundary overview operation whose database query count does not grow per Skill.

#### Scenario: Load many Skills
- **WHEN** the overview contains many Skills
- **THEN** native loading SHALL use batch binding queries rather than one query or IPC request per Skill

### Requirement: Existing Skill database upgrade compatibility
The system SHALL create all Skill binding storage required by a pending reliability migration before that migration cleans or indexes binding rows.

#### Scenario: Upgrade a database created before API Skill bindings
- **WHEN** an existing database has recorded migrations 1 through 36 but does not contain the API-Agent Skill binding table
- **THEN** migration 37 SHALL create the missing table before cleanup
- **AND** SHALL complete without deleting Skill source records or directories

### Requirement: Behavioral Web adapter parity
The Web/mock adapter SHALL preserve Skill documents and enforce the same identity, scope, binding-type, lifecycle, validation, and cleanup outcomes as the native adapter, while simulating filesystem-only effects deterministically.

#### Scenario: Web create update and preview
- **WHEN** a Web user creates or updates a Skill and then previews it
- **THEN** the preview SHALL contain the submitted metadata and body

#### Scenario: Web delete cleanup
- **WHEN** a Web user deletes a Skill
- **THEN** both CLI and API mock bindings for that Skill SHALL be removed

### Requirement: External Skill import
The system SHALL import external Skills by copying the external Skill directory into the selected scope's managed Skill source directory.

#### Scenario: Import valid external Skill
- **WHEN** a user imports an external directory containing a valid `SKILL.md`
- **THEN** the system SHALL copy it into the selected scope, create a registry record, and make it available for Agent binding

#### Scenario: Import invalid external Skill
- **WHEN** a user imports an external directory without valid required `SKILL.md` metadata
- **THEN** the system SHALL reject the import and SHALL NOT create a registry record

### Requirement: Skill drift detection
The system SHALL detect drift between SQLite registry records, source `SKILL.md` files, and CLI Agent mount paths.

#### Scenario: Source file changed
- **WHEN** a Skill source `SKILL.md` content hash differs from the registry hash
- **THEN** the system SHALL report metadata or content drift for that Skill

#### Scenario: Registry missing for source Skill
- **WHEN** a managed Skill source directory exists with `SKILL.md` but no registry record exists for the selected scope
- **THEN** the system SHALL report an unregistered Skill drift issue

#### Scenario: Missing mount
- **WHEN** an enabled Skill is bound to a CLI Agent but no managed link exists in that Agent's mount path
- **THEN** the system SHALL report a missing mount drift issue

#### Scenario: Conflicting mount target
- **WHEN** a file, directory, or foreign link occupies a bound Skill target path
- **THEN** the system SHALL report a conflict drift issue for that Skill and CLI Agent

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

### Requirement: Safe CLI Skill mount roots
The system SHALL preflight every existing component of a CLI Agent's configured Skill mount root without following linked components before it creates, repairs, or migrates a managed per-Skill link.

#### Scenario: Use an existing normal mount root
- **WHEN** every existing component of the configured mount root is a normal directory
- **THEN** the system SHALL create or repair the requested managed per-Skill link through the existing binding transaction

#### Scenario: Create an absent normal mount root
- **WHEN** the configured mount root or one of its normal descendants does not exist and no existing ancestor is linked
- **THEN** the system SHALL create the required normal directories before creating the managed per-Skill link

#### Scenario: Reject a live external directory link
- **WHEN** the configured mount root or an existing component below the canonical scope root is a symlink, junction, or reparse point that resolves to a directory
- **THEN** the system SHALL reject the binding with an actionable error identifying the stable Agent id
- **AND** SHALL NOT follow, delete, replace, or write through the external directory link

#### Scenario: Reject a broken directory link
- **WHEN** the configured mount root or an existing component below the canonical scope root is a symlink, junction, or reparse point whose target is missing or unavailable
- **THEN** the system SHALL reject the binding with an actionable broken-link error identifying the stable Agent id
- **AND** SHALL NOT delete or replace the broken link

#### Scenario: Preserve state after mount-root rejection
- **WHEN** mount-root preflight rejects a CLI Skill assignment
- **THEN** the system SHALL leave the Skill source, current CLI/API Agent assignments, SQLite records, external link, and external target unchanged

### Requirement: Agent-specific Skill binding diagnostics
The system SHALL write CLI Skill bind and unbind results through the unified logging service with safe stable-Agent context and without raw home or external target paths.

#### Scenario: Record rejected mount-root binding
- **WHEN** a CLI Skill binding fails mount-root preflight
- **THEN** the unified error log SHALL include the binding action, Skill id, and stable Agent id
- **AND** SHALL NOT include the absolute mount-root path or external link target

### Requirement: Unregistered Skill sources are repairable
The system SHALL resolve an `UnregisteredSource` drift issue by adopting the existing source into the registry, so that a source directory present on disk without a registry record does not remain permanently unusable.

#### Scenario: Synchronization adopts an unregistered source
- **WHEN** Skill synchronization runs and reports an `UnregisteredSource` issue for a source directory
- **THEN** the system SHALL register that source and clear the issue
- **AND** the Skill SHALL become listable, bindable, and mountable like any other registered Skill

#### Scenario: Adoption does not resurrect an intentionally deleted built-in
- **WHEN** an unregistered source belongs to a built-in Skill the user has deleted
- **THEN** the system SHALL leave it unregistered
- **AND** the existing intentional-deletion behavior SHALL continue to apply

#### Scenario: A failed adoption is reported rather than retried forever
- **WHEN** adopting an unregistered source fails
- **THEN** the system SHALL report the failure with its reason
- **AND** it SHALL NOT leave the user without a way to see why the Skill is absent

