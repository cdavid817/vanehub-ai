# agent-code-review Specification

## Purpose
TBD - created by archiving change add-agent-code-review-center. Update Purpose after archive.
## Requirements
### Requirement: Recoverable review lifecycle
The system SHALL create and recover a bounded Review Session linked to the originating session and canonical workspace or worktree, with base/head witnesses, working-tree fingerprint, status, files, comments, findings, decisions, and timestamps persisted in SQLite.

#### Scenario: Open review after Agent edits
- **WHEN** a session with three changed text files opens Review Changes
- **THEN** the service SHALL create or recover one active review and list the three files from a fresh bounded workspace snapshot

#### Scenario: Restart during review
- **WHEN** the application restarts with an unfinished persisted review
- **THEN** the review SHALL recover its comments, findings, and decisions and SHALL revalidate its workspace witnesses before presenting current anchors

### Requirement: Bounded review diff loading
The review service SHALL load structured diffs through `workspaces`, preserve tracked, added, deleted, reliably detected renamed, and bounded untracked text changes, and SHALL expose binary, oversized, per-file, total, and truncation states without unbounded reads.

#### Scenario: Select a bounded text file
- **WHEN** the user selects a changed text file within the configured bounds
- **THEN** the service SHALL return structured files, hunks, lines, line numbers, fingerprints, and summary counts using UTF-8-safe content

#### Scenario: Select binary or oversized content
- **WHEN** the selected file is binary or exceeds its file budget
- **THEN** the service SHALL return metadata and a binary or oversized marker without returning unrestricted bytes or text diff content

#### Scenario: Aggregate diff exceeds its budget
- **WHEN** the review snapshot exceeds the total diff budget
- **THEN** the service SHALL retain the changed-file summary and load eligible files individually rather than materializing the whole diff

### Requirement: Fingerprinted review comments
Each review comment SHALL retain a bounded body, normalized relative file path, side, line range, hunk fingerprint, context fingerprint, status, and stale state; anchors MUST NOT depend on absolute line number alone.

#### Scenario: Comment on an added line
- **WHEN** the user comments on an added line
- **THEN** the persisted comment SHALL retain the added side, new-line range, hunk/context fingerprints, and body

#### Scenario: Agent changes the same file
- **WHEN** a later snapshot no longer matches the comment's exact anchor
- **THEN** the service SHALL uniquely relocate the anchor from bounded same-file context or mark it stale when relocation is absent or ambiguous

#### Scenario: Resolve a comment
- **WHEN** the user resolves an active comment
- **THEN** the review SHALL preserve the comment and transition its status rather than deleting its audit history

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

### Requirement: Guarded destructive revert
Whole-file and hunk revert SHALL require explicit confirmation or permission, canonical workspace confinement, current file/worktree witness equality, and atomic fail-closed patch application with no fuzzy match.

#### Scenario: Revert one current hunk
- **WHEN** the user confirms a hunk revert and all snapshot witnesses still match
- **THEN** the native runtime SHALL reverse only that hunk and preserve unrelated changes in the same file

#### Scenario: Workspace changed after snapshot
- **WHEN** an external edit changes the expected witness before revert
- **THEN** the runtime SHALL reject the revert as stale without modifying any file

#### Scenario: Malicious path supplied
- **WHEN** a revert request uses traversal, absolute path, symlink escape, or a path outside the owning session root
- **THEN** the runtime SHALL reject it before reading or modifying the target

### Requirement: Structured feedback to the originating Agent
The service SHALL send selected comments as a provider-neutral structured review feedback envelope through the existing session/Agent boundary, preserving file, side, line, hunk, decision, and stale metadata without UI-owned provider payload construction.

#### Scenario: Send selected comments
- **WHEN** the user selects comments on two files and sends feedback
- **THEN** the originating session SHALL receive a bounded numbered review feedback message with the structured metadata retained

#### Scenario: Selected anchor is stale
- **WHEN** selected feedback includes a stale anchor
- **THEN** the service SHALL require explicit acknowledgement and label the anchor stale rather than silently presenting its old lines as current

### Requirement: Pluggable automated review actions
The Review Center SHALL offer allowlisted Review Agent, Tests, and Security Checks actions through existing Agent/tool/operation runtimes and SHALL normalize terminal results into bounded review findings.

#### Scenario: Tests produce findings
- **WHEN** the Tests action completes with structured failures
- **THEN** the review SHALL persist bounded findings with severity, title, source, optional anchor, operation reference, and status

#### Scenario: Automated output is invalid
- **WHEN** an action cannot produce valid bounded findings
- **THEN** the operation SHALL fail with page-visible bounded output and SHALL NOT fabricate findings

### Requirement: Review observability and data minimization
Review lifecycle actions SHALL emit redacted metadata-only events through operations and unified logging, and MUST NOT persist code, full diffs, comment/finding bodies, prompts, secrets, or raw tool output in diagnostic logs.

#### Scenario: Log a stale revert rejection
- **WHEN** a revert is rejected because its witness is stale
- **THEN** the log SHALL contain safe ids, counts, operation id, outcome category, and timing without code or review prose

### Requirement: Runtime parity with honest Web simulation
The frontend service contract SHALL be implemented by both Tauri and Web/mock adapters with equivalent review states and DTO semantics; Web mutations SHALL be deterministic simulations explicitly marked as simulated.

#### Scenario: Revert in Web mode
- **WHEN** a Web/mock user reverts a fixture hunk
- **THEN** only in-memory fixture state SHALL change, the receipt SHALL say it is simulated, and the UI SHALL NOT claim a real Git write

#### Scenario: Read real Tauri diff
- **WHEN** a desktop review opens a test repository with working-tree changes
- **THEN** the Tauri adapter SHALL return the real bounded changed-file snapshot through declared native commands

### Requirement: Responsive and accessible Review Center
The Changes tab SHALL provide changed-file navigation, summaries, unified/split diff, line numbers, inline comment editing, previous/next file, stale notices, copy, accept/revert, feedback and automated actions, plus loading/error/empty states in both registered styles and desktop/narrow layouts.

#### Scenario: Use desktop layout
- **WHEN** the Review Center renders at desktop width
- **THEN** a changed-file rail and main diff region SHALL remain simultaneously usable without comments obscuring code

#### Scenario: Use narrow layout
- **WHEN** the Review Center renders at narrow width
- **THEN** the file rail SHALL be collapsible or switchable, the editor SHALL remain usable, and code overflow SHALL remain recoverable within the diff region

#### Scenario: Use either visual style
- **WHEN** `futuristic` or `minimal` is active
- **THEN** equivalent semantic tokens SHALL provide readable focus, selection, status, diff, warning, and action states without layout shifts

### Requirement: Linear bounded processing
Native diff parsing, fingerprinting, anchor matching, and frontend row construction SHALL operate in linear time relative to accepted bounded input and SHALL avoid rebuilding every diff row for an unrelated comment-state change.

#### Scenario: Exercise maximum accepted diff fixture
- **WHEN** deterministic benchmark fixtures reach configured file, byte, hunk, and line bounds
- **THEN** structural measurements SHALL demonstrate one-pass bounded processing with no quadratic nested scan over all lines

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

