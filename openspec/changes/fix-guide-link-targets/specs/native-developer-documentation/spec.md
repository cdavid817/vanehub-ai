## ADDED Requirements

### Requirement: Every documentation file is reachable

A Markdown document committed under `docs/` SHALL be reachable from a guide's navigation or from a documentation entry point. An unreferenced document SHALL be treated as a defect, not as archived material, because nothing distinguishes it from a document that was forgotten.

Where a document records a point-in-time survey rather than current behavior, it SHALL be labeled as such where it is linked, including the revision it was written against.

#### Scenario: A document is referenced from nowhere

- **WHEN** a Markdown file under `docs/` appears in no `SUMMARY.md`, no README, and no other document
- **THEN** it SHALL be treated as a defect to resolve by linking it, folding it in, or removing it

#### Scenario: A point-in-time document is linked

- **WHEN** a document describes the system as of a specific revision rather than as maintained narrative
- **THEN** the link to it SHALL state that it is a snapshot and name that revision
- **AND** it SHALL NOT be presented alongside current chapters as though it were maintained
