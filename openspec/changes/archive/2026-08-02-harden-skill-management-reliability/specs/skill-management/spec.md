## MODIFIED Requirements

### Requirement: Agent mount path management
The system SHALL use registered CLI-capable Agent ids as Skill mount carriers, SHALL store one editable mount path per CLI-capable Agent, and SHALL reject mount paths that overlap the VaneHub-managed `.vanehub` namespace or any Skill source directory.

#### Scenario: Default Agent mount paths
- **WHEN** the system returns mount paths for registered Agents
- **THEN** it SHALL include defaults for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode` when those CLI Agents are registered
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

#### Scenario: Bind Skill to Agent
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

## ADDED Requirements

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
