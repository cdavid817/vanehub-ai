## MODIFIED Requirements

### Requirement: Review decision and hunk acceptance

The review SHALL track `pending`, `accepted`, and `changes_requested` decisions independently at review and witnessed-hunk scope. A hunk decision SHALL identify the review, normalized relative path, hunk fingerprint, and expected snapshot fingerprint. Recording any review or hunk decision SHALL NOT stage, unstage, commit, revert, rewrite, or otherwise modify the Git index or working tree.

#### Scenario: Accept a hunk

- **WHEN** the user accepts a hunk whose review, file, hunk, and snapshot witnesses remain current
- **THEN** the service SHALL persist `accepted` only for that witnessed hunk
- **AND** the review-level decision and every other hunk decision SHALL remain unchanged
- **AND** the Git index and working tree SHALL remain unchanged

#### Scenario: Request changes on a current hunk

- **WHEN** the user records `changes_requested` for one current hunk
- **THEN** the service SHALL persist that hunk decision independently from the review-level decision
- **AND** it SHALL retain the decision as review evidence without applying a Git mutation

#### Scenario: Accept the whole review

- **WHEN** the user accepts the current Review Session
- **THEN** the service SHALL update only the review-level decision against the expected Review Session version
- **AND** it SHALL NOT rewrite individual hunk decisions or mutate Git content

#### Scenario: Hunk witness is stale

- **WHEN** the expected snapshot or hunk fingerprint no longer matches the current bounded review snapshot
- **THEN** the hunk decision operation SHALL return `stale_witness`
- **AND** it SHALL preserve the prior review and hunk decisions without modifying Git content

## ADDED Requirements

### Requirement: Witnessed standard review patch generation

The Review Center SHALL obtain a standard bounded Git patch through the native review/workspace service for a current Review Session, file, or hunk witness, independently from copying the currently rendered diff lines.

#### Scenario: Copy a current file patch

- **WHEN** the user requests a standard patch for a current bounded text-file witness
- **THEN** the native service SHALL return a patch containing the required file headers and hunk headers plus a patch fingerprint
- **AND** applying the patch to the declared base fixture with `git apply --check` SHALL succeed

#### Scenario: Copy a current hunk patch

- **WHEN** the user requests a standard patch for one current hunk
- **THEN** the service SHALL render only the witnessed hunk with sufficient valid patch headers and context
- **AND** it SHALL NOT include unrelated hunks from the same file

#### Scenario: Patch witness is stale or unsupported

- **WHEN** the review snapshot changed or the requested content is binary, oversized, truncated, or otherwise unable to form an exact bounded patch
- **THEN** the service SHALL reject the request with a typed reason
- **AND** the UI SHALL NOT copy an obsolete or syntactically incomplete patch

#### Scenario: Copy displayed lines

- **WHEN** the user chooses Copy Displayed Lines rather than Copy Standard Patch
- **THEN** the UI MAY copy the visible textual representation
- **AND** it SHALL label that action separately so it is not represented as an apply-ready patch

### Requirement: Review file Viewed state and progress

The Review Session SHALL persist a per-file Viewed state against the current file snapshot and SHALL expose bounded review progress for current changed files.

#### Scenario: Mark a current file Viewed

- **WHEN** the user marks a changed file Viewed against its current snapshot fingerprint
- **THEN** the review SHALL persist that state and update `viewed current files / current changed files` progress

#### Scenario: File content changes after it was Viewed

- **WHEN** a later review snapshot changes the file fingerprint
- **THEN** the file SHALL return to unviewed for the new snapshot
- **AND** the historical Viewed action MAY remain in review evidence without being presented as current approval

#### Scenario: Display review progress

- **WHEN** Review Center renders a current Review Session
- **THEN** it SHALL show current Viewed progress plus unresolved comment and finding counts
- **AND** status SHALL remain identifiable without color alone

### Requirement: Review execution-evidence correlation

Review decisions, automated actions, findings, and guarded mutations SHALL retain safe run, trace, span, operation, Agent, and seat correlation when supplied by their canonical execution source.

#### Scenario: Test finding originates from an execution span

- **WHEN** an automated Test action persists a finding with run, operation, and span correlation
- **THEN** Review Center SHALL expose a link to that execution evidence
- **AND** the session-run report SHALL count the finding through its canonical review/verification summary

#### Scenario: Review event has no execution correlation

- **WHEN** a user manually changes a review decision outside an execution run
- **THEN** the review SHALL persist the decision without fabricating run, trace, or span ids

#### Scenario: Persist correlated review diagnostics

- **WHEN** review lifecycle metadata is written to unified logging or execution evidence
- **THEN** it SHALL contain only safe ids, fingerprints, counts, timing, decision/status classifications, and operation correlation
- **AND** it SHALL omit code, full patch text, comment/finding bodies, prompts, secrets, and raw tool output
