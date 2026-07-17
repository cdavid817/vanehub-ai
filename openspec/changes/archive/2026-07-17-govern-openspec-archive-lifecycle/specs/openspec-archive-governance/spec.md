## ADDED Requirements

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
