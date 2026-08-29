## Purpose

Govern the curated developer documentation for VaneHub AI: English and Simplified Chinese mdBook developer guides carrying the project's architectural narrative in both languages, a complementary Rustdoc API reference, a reproducible deterministic build, and documentation validation. Historical and ADR-style architectural content lives in `src-tauri/ARCHITECTURE.md` rather than a competing narrative directory, dated working artifacts are kept out of the published documentation tree, and an unreferenced document fails the documentation gate rather than accumulating unnoticed.

## MODIFIED Requirements

### Requirement: Curated mdBook developer guide
The repository SHALL provide English and Simplified Chinese mdBook developer guides covering system architecture, frontend service boundaries, native bounded contexts, persistence and migration ownership, unified logging, testing, packaging, and contribution workflows. The developer guide SHALL be the project's architectural narrative in both languages, and the two SHALL carry the same chapters under the same navigation order and the same section structure within each chapter.

Neither language SHALL be a condensed digest of the other. Where one language documents a subject with a named section, the other SHALL document the same subject with the corresponding section.

Historical or ADR-style architectural content SHALL live in `src-tauri/ARCHITECTURE.md` or in a clearly labeled historical section of the developer guide, not in a competing `docs/architecture/` narrative directory.

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

#### Scenario: Both languages carry the same sections
- **WHEN** a chapter exists in both developer guides
- **THEN** the two SHALL expose the same named sections covering the same subjects
- **AND** a chapter that documents in one language a type inventory, a request path, or a lifecycle that the other language omits SHALL be treated as a defect in the shorter chapter

#### Scenario: A chapter changes in one language
- **WHEN** a developer-guide chapter gains, loses, or restructures a section in one language
- **THEN** the corresponding chapter in the other language SHALL receive the equivalent change in the same change

### Requirement: Documentation validation
The documentation pipeline SHALL validate Markdown links, README parity, mdBook navigation, document reachability, supported Rust code samples, Rustdoc generation, and documentation output assembly.

#### Scenario: Detect a broken internal link
- **WHEN** an authored guide or assembled output references a missing repository-relative chapter or asset
- **THEN** the documentation check SHALL fail and identify the source file and invalid target

#### Scenario: Validate code samples
- **WHEN** `mdbook test` encounters a Rust sample marked as testable
- **THEN** the sample SHALL compile and execute according to mdBook semantics
- **AND** illustrative or environment-dependent samples SHALL be explicitly marked rather than failing unpredictably

#### Scenario: Keep selected API boundaries documented
- **WHEN** a selected native documentation boundary gains or changes an exposed contract item
- **THEN** the documentation checks SHALL require the boundary inventory and its Rust documentation to remain complete

### Requirement: Every documentation file is reachable

A Markdown document committed under `docs/` SHALL be reachable from a guide's navigation or from a documentation entry point. An unreferenced document SHALL be treated as a defect, not as archived material, because nothing distinguishes it from a document that was forgotten.

Reachability SHALL be enforced by an automated check in the repository's documentation gate rather than by review alone. The check SHALL treat each guide's `SUMMARY.md` and the repository entry-point documents as roots, and SHALL report every committed document under `docs/` that no root reaches.

Where a document records a point-in-time survey rather than current behavior, it SHALL be labeled as such where it is linked, including the revision it was written against.

#### Scenario: A document is referenced from nowhere

- **WHEN** a Markdown file under `docs/` appears in no `SUMMARY.md`, no README, and no other document
- **THEN** it SHALL be treated as a defect to resolve by linking it, folding it in, or removing it

#### Scenario: A point-in-time document is linked

- **WHEN** a document describes the system as of a specific revision rather than as maintained narrative
- **THEN** the link to it SHALL state that it is a snapshot and name that revision
- **AND** it SHALL NOT be presented alongside current chapters as though it were maintained

#### Scenario: Reachability regresses

- **WHEN** a change commits a document under `docs/` that no root reaches, or removes the last link to an existing one
- **THEN** the documentation gate SHALL fail and name the unreachable document
- **AND** the failure SHALL NOT depend on a reviewer noticing the missing link

#### Scenario: A document is reachable only through another unreachable document

- **WHEN** a set of documents under `docs/` links only to each other and no root reaches any of them
- **THEN** the check SHALL report every document in that set rather than treating their mutual links as reachability
