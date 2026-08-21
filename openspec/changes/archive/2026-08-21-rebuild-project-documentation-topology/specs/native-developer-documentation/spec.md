## MODIFIED Requirements

### Requirement: Curated mdBook developer guide
The repository SHALL provide an English mdBook developer guide covering system architecture, frontend service boundaries, native bounded contexts, persistence and migration ownership, unified logging, testing, packaging, and contribution workflows. The developer guide SHALL be the single English architectural narrative for the project. Historical or ADR-style architectural content SHALL live in `src-tauri/ARCHITECTURE.md` or in a clearly labeled historical section of the developer guide, not in a competing `docs/architecture/` narrative directory.

#### Scenario: Developer navigates the guide
- **WHEN** a developer builds or opens the developer guide
- **THEN** its summary SHALL expose the documented architecture and maintenance topics through stable chapter paths
- **AND** existing authoritative repository documents SHALL be linked or included instead of copied into conflicting narratives

#### Scenario: Explain runtime differences
- **WHEN** a documented behavior differs between Tauri desktop and Web/mock runtime
- **THEN** the guide SHALL identify both behaviors and SHALL preserve the React service and adapter boundary

#### Scenario: Locate architectural decisions
- **WHEN** a maintainer looks for an architecture decision record
- **THEN** the developer guide SHALL point to `src-tauri/ARCHITECTURE.md` as the ADR source of truth
- **AND** no second competing architectural narrative directory SHALL exist under `docs/`

## ADDED Requirements

### Requirement: Working artifacts are kept out of the published documentation tree
Dated working artifacts such as plans and exploratory specs SHALL NOT be published under the `docs/` documentation tree. They SHALL live in a non-published working-artifacts location so that documentation consumers see only curated, reviewed material.

#### Scenario: Working artifact is created
- **WHEN** a dated plan or exploratory design artifact is produced during development
- **THEN** it SHALL NOT be placed under `docs/`
- **AND** the documentation build and link validator SHALL NOT be required to include it
