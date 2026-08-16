## ADDED Requirements

### Requirement: Witnessed review snapshots and guarded Git mutation
The workspace service SHALL extend its confined structured Git inspection with stable review file/hunk fingerprints and explicitly guarded whole-file and hunk revert operations that validate the owning session root and current witnesses immediately before mutation.

#### Scenario: Create review snapshot
- **WHEN** a review requests the session's changed files
- **THEN** `workspaces` SHALL produce bounded structured metadata and deterministic content witnesses without persisting full diff content

#### Scenario: Apply reverse hunk
- **WHEN** a confirmed reverse-hunk request has a current witness and an exact patch target
- **THEN** `workspaces` SHALL apply only that patch under its mutation guard and return the resulting witness

#### Scenario: Refuse unsafe mutation
- **WHEN** path confinement, file type, size, fingerprint, or exact patch application validation fails
- **THEN** `workspaces` SHALL fail closed without partial writes
