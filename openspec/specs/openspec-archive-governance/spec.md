# openspec-archive-governance Specification

## Purpose
TBD - created by archiving change govern-openspec-archive-lifecycle. Update Purpose after archive.
## Requirements
### Requirement: Canonical OpenSpec archive location
The repository SHALL store completed OpenSpec change directories at `openspec/changes/archive/YYYY-MM-DD-<change-name>/`, and contributor guidance SHALL reference that location.

#### Scenario: Contributor archives a completed change
- **WHEN** a contributor runs the normal OpenSpec archive workflow for a completed change
- **THEN** the completed change directory is located under `openspec/changes/archive/` with its date-prefixed change name

### Requirement: Archive admission checks
The repository SHALL archive a normal implementation change only after its tasks are complete, `openspec validate <change-name> --strict` succeeds, and implementation verification is recorded when code changed. The workflow MUST NOT use `--no-validate`; it MAY use `--skip-specs` only when the change has no main-spec impact.

#### Scenario: Archive a validated implementation change
- **WHEN** a completed implementation change has passed strict validation and recorded its verification outcome
- **THEN** the contributor may run `openspec archive <change-name>` without `--no-validate`

#### Scenario: Archive a documentation-only change
- **WHEN** a completed change has no main-spec impact
- **THEN** the contributor may use `openspec archive <change-name> --skip-specs` after strict validation succeeds

### Requirement: Searchable archive index
The repository SHALL maintain `openspec/changes/archive/README.md` as a generated index of the archive directories and SHALL regenerate it immediately after archiving a change or completing a cold-archive migration.

#### Scenario: Generate the archive index
- **WHEN** the archive index generator runs against the repository archive directory
- **THEN** the resulting README lists every date-prefixed archive directory that remains in the online archive

### Requirement: Archive retention and cold migration
The repository SHALL retain complete Markdown artifacts in the online archive and SHALL review them at least every six months before any cold migration. A cold migration MUST preserve the archived content in a Git repository, immutable branch, or tag and MUST record a verifiable destination reference before removing the online copy. The repository MUST NOT replace online archive directories with zip or tar files as the normal retention mechanism.

#### Scenario: Migrate an eligible archive to cold storage
- **WHEN** a maintainer migrates a completed archive after a six-month review
- **THEN** the maintainer verifies the destination Git reference and records it before removing the online directory

### Requirement: Structured archive catalog
The repository SHALL maintain `openspec/changes/archive/archive-index.json` as a generated, deterministic catalog of online archive directories. Each catalog entry SHALL include the archive date, change name, relative archive path, available top-level planning artifacts, and capability names derived from delta-spec directory paths. Archive discovery tools and contributor guidance MUST query this catalog before scanning archived Markdown content.

#### Scenario: Locate archives for a capability
- **WHEN** a maintainer needs the history for a capability
- **THEN** the maintainer can filter `archive-index.json` by that capability without traversing archive directories or searching their Markdown content

#### Scenario: Regenerate the catalog after archiving
- **WHEN** the archive index generator runs after an archive operation
- **THEN** it writes one catalog entry for every date-prefixed online archive directory and refreshes the human-readable README from the same entry set
