## ADDED Requirements

### Requirement: Workspace inspection cancellation is generation-safe and RAII-owned

Each cancellable workspace inspection SHALL register a unique generation under its search id and SHALL own that registration through an RAII-equivalent async guard. Superseding a search SHALL cancel the previous generation, and completion, failure, abort, or drop of one generation MUST NOT remove or mutate a newer generation.

#### Scenario: Older search finishes after its replacement starts

- **WHEN** search A is registered, search B starts with the same search id and supersedes A, and A finishes afterward
- **THEN** A SHALL remove only its own generation if still present
- **AND** B SHALL remain registered and cancellable
- **AND** A's late result SHALL NOT overwrite B's current frontend state

#### Scenario: Owning future is aborted

- **WHEN** the async future that owns a running inspection is aborted or dropped before normal completion
- **THEN** its registration guard SHALL signal the worker cancellation token
- **AND** the worker SHALL stop at a bounded checkpoint
- **AND** registry/admission cleanup SHALL occur after that worker exits without requiring an explicit finish call

### Requirement: Inspection budgets account for actual work and report completeness

Every recursive or potentially large workspace inspection SHALL enforce finite budgets for directories, entries, files, bytes, metadata/canonicalization work, retained candidates, results, depth, and a monotonic deadline as applicable. Budget accounting SHALL include ignored, unreadable, non-matching, and rejected entries that still consumed work.

Results SHALL distinguish `Complete`, `Partial`, and `Unavailable` and SHALL include a stable primary reason code whenever coverage is not complete.

#### Scenario: Many non-matching entries exhaust work budget

- **WHEN** a traversal visits the configured entry limit without producing a match
- **THEN** it SHALL stop before visiting another entry
- **AND** it SHALL return an empty `Partial` result with `entry_budget_exhausted`
- **AND** the UI SHALL NOT present that result as definitive proof that no match exists

#### Scenario: User cancels during a file read

- **WHEN** cancellation is signalled while content search is reading a large eligible file
- **THEN** the worker SHALL observe cancellation at a bounded chunk checkpoint
- **AND** it SHALL return or terminate with `cancelled` or `superseded` coverage for that generation

#### Scenario: Traversal completes under policy and budget

- **WHEN** every eligible entry under the effective recursive ignore policy is exhausted without omission, failure, cancellation, or budget stop
- **THEN** coverage SHALL be `Complete`
- **AND** an empty result MAY be presented as no matches for that request

### Requirement: Recursive content and document inspection use streaming bounded memory

Content search and recursive document discovery SHALL process eligible entries as a bounded stream and MUST NOT first materialize the complete candidate-file set. Their retained memory SHALL be limited to traversal state, current bounded file/chunk state, bounded results/snippets, and bounded coverage/error counters.

#### Scenario: Workspace contains more candidates than the candidate budget

- **WHEN** a workspace contains a very large number of eligible files
- **THEN** content search SHALL open/process candidates incrementally
- **AND** instrumentation SHALL show no full candidate vector proportional to workspace size
- **AND** cancellation, deadline, and byte/file budgets SHALL remain enforceable during traversal

#### Scenario: Default document discovery encounters dependency output

- **WHEN** recursive document discovery reaches `node_modules`, `target`, `dist`, or another configured generated/dependency directory without an explicit include override
- **THEN** the shared recursive ignore policy SHALL skip descent
- **AND** the skipped subtree SHALL not consume child-entry/file-read work beyond the directory entry required to apply the policy

#### Scenario: User explicitly navigates to an ignored directory

- **WHEN** a user directly lists or reads a path that is ignored only by recursive discovery policy
- **THEN** the system SHALL continue to allow that explicit operation subject to existing root, authorization, type, and size rules
- **AND** it SHALL NOT treat ignore policy as an access-control denial

### Requirement: Path and directory pagination retain only bounded selections

Path search and immediate directory pagination SHALL use a bounded selection algorithm whose retained candidate/page memory is proportional to configured result/page limits. They MUST NOT retain and sort every eligible entry solely to return one bounded page.

#### Scenario: Directory contains hundreds of thousands of entries

- **WHEN** the user requests a page of `N` entries from a very large immediate directory
- **THEN** the implementation SHALL retain at most `N + 1` selected entries plus fixed traversal overhead
- **AND** all scanned entries SHALL still consume entry/metadata budgets
- **AND** incomplete scanning SHALL be reported separately from normal `has_more` pagination

#### Scenario: Directory changed between pages

- **WHEN** a versioned cursor's detectable directory fingerprint, directory identity, order mode, or policy identity no longer matches the next request
- **THEN** the adapter SHALL return `invalid_cursor` or `stale_cursor`
- **AND** the frontend SHALL restart and replace pagination rather than append an incompatible page

### Requirement: Workspace inspection adapters preserve admission and coverage parity

Local, remote, Tauri, and Web/mock implementations SHALL expose the same generation, coverage state, reason-code, cursor, and budget-summary semantics. Global and per-workspace admission SHALL be acquired before blocking or remote work starts and SHALL remain held until that work actually exits.

#### Scenario: Per-workspace inspection capacity is exhausted

- **WHEN** another independent inspection cannot obtain a per-workspace permit within the finite admission policy
- **THEN** it SHALL return `Unavailable` with `inspection_busy`
- **AND** it SHALL NOT enqueue an unbounded blocking task or launch a remote provider process

#### Scenario: Web mock hits a byte budget

- **WHEN** deterministic Web/mock fixtures consume their configured simulated byte budget
- **THEN** Web/mock SHALL return the same Partial/byte-budget reason contract as native adapters
- **AND** it SHALL clearly remain simulated and SHALL NOT claim a native filesystem scan occurred
