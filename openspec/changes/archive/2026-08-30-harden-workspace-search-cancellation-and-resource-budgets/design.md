## Context

The `workspaces` context owns local and remote project inspection: directory listing, path search, content search, document discovery, cancellation, and the completeness semantics presented to the user. Runtime/API code owns admission to blocking or external-provider execution. Infrastructure owns filesystem walking, metadata/read operations, remote commands, and provider-specific cancellation.

The current cancellation shape resembles:

```text
A: begin(id) -> token A
B: begin(id) -> replace with token B; cancel token A
A: finish(id) -> remove id unconditionally
B: continues, but cancel(id) no longer reaches B
```

Normal cleanup also depends on an explicit call after `await`; abort/drop/panic paths can bypass it.

The current resource controls are mostly output-oriented. A small result cap does not constrain unsuccessful entries, directories, filesystem calls, bytes, retained candidate vectors, queue depth, or time. This design creates one application-level execution contract shared by local, remote, Tauri, and Web/mock implementations.

## Goals / Non-Goals

**Goals:**

- Make cancellation registration correct under same-id supersession, explicit cancellation, normal completion, future abort/drop, worker error, and panic unwind.
- Bound actual filesystem/provider work and retained memory independently of result count.
- Ensure cancellation/deadline checks occur frequently enough that bounded workers stop cooperatively rather than running to natural completion.
- Prevent repeated search requests from unboundedly occupying the blocking pool or launching remote work.
- Distinguish complete results from partial, cancelled, superseded, budget-exhausted, unreadable, stale-cursor, busy, and unavailable results.
- Preserve one frontend service contract and stable reason codes across native local, native remote, and Web/mock adapters.
- Use deterministic structural tests instead of flaky wall-clock benchmarks.

**Non-Goals:**

- Filesystem-handle confinement or a security claim against concurrent symlink/reparse replacement.
- Persistent indexing or exact concurrent filesystem snapshots.
- Search-language changes, regex additions, semantic ranking, LSP/Tree-sitter integration, Git search, or terminal FTS changes.
- Increasing current output/file-size limits.
- Applying recursive ignore defaults to explicit direct reads or direct directory navigation.

## Invariants

1. One active cancellation slot is identified by `(search_id, generation)`.
2. Completing or dropping generation `n` can remove only generation `n`; it cannot remove generation `n + 1`.
3. Superseding a slot signals the old token before the new request begins expensive work.
4. Dropping/aborting the async owner signals its token even if normal finish code does not run.
5. Admission is acquired before `spawn_blocking` or remote process/channel launch.
6. Every accounted operation checks and consumes its budget before exceeding the configured limit.
7. Result count is only one budget dimension; unsuccessful work is also counted.
8. Recursive inspection retains memory proportional to explicit page/result/candidate limits, not total workspace size.
9. `Complete` means traversal under the selected ignore policy was exhausted without cancellation, budget stop, unreadable omission, provider failure, or cursor incompatibility.
10. A partial/unavailable result carries a stable reason code and does not masquerade as “no matches.”
11. Deadlines use an injected monotonic clock for process-local work.
12. Local, remote, and Web/mock return semantically equivalent state even though Web/mock performs no native scan.
13. Ignore policy affects discovery scope, not authorization; root confinement and explicit-access rules still apply separately.
14. A stale result generation cannot overwrite current UI state when the frontend requested a newer generation.

## Decisions

### 1. Generation-qualified RAII cancellation registration

Replace id-only start/finish with an application-owned registry:

```text
SearchId(String)
SearchGeneration(u64)

SearchSlot:
  generation
  token: CancellationToken

SearchRegistration:
  search_id
  generation
  token
  registry_ref
  completion_state
```

`begin(search_id)` executes under one synchronization boundary:

1. allocate the next non-zero generation for that id;
2. replace the current slot with the new `(generation, token)`;
3. signal the previous token as `Superseded`;
4. return the `SearchRegistration` guard.

The guard exposes:

```text
registration.token()
registration.generation()
registration.complete(outcome)
```

`complete` compare-removes only when the registry still contains the same generation and token identity. `Drop` signals the guard's own token as cancelled/owner-dropped and performs the same compare-remove. Setting an already completed token is harmless, but implementation should avoid reporting normal completion as user cancellation.

Explicit `cancel(search_id)` signals only the currently registered generation. Optional `cancel(search_id, generation)` can be used internally/frontend-side where a precise request token already exists.

Generation allocation handles integer wrap by skipping zero and any currently occupied generation; tests use an injectable allocator to exercise wrap behavior.

### 2. Async owner and blocking worker relationship

The RAII guard remains in the async service future that owns the request. The blocking/local/remote worker receives only a clone of the cancellation token, generation, immutable request, and budget tracker.

```text
acquire admission
→ begin registration
→ spawn_blocking / launch remote provider with token + generation
→ await result
→ registration.complete(result outcome)
→ release admission
```

If the future is aborted or dropped:

- the registration guard signals cancellation;
- the blocking worker observes the token at defined checkpoints and exits;
- the admission permit remains owned until the worker completion future releases it, preventing hidden work from being treated as free;
- late result delivery is discarded by generation comparison.

No blocking worker mutates the cancellation registry directly.

### 3. Shared inspection budget

Define application/domain value objects:

```text
WorkspaceInspectionBudgetLimits:
  max_directories_visited
  max_entries_visited
  max_files_opened
  max_bytes_read
  max_metadata_or_canonicalization_ops
  max_retained_candidates
  max_results
  max_depth
  deadline

WorkspaceInspectionBudgetTracker:
  atomic/local counters
  monotonic start/deadline
  cancellation token

BudgetStopReason:
  DirectoryBudgetExhausted
  EntryBudgetExhausted
  FileBudgetExhausted
  ByteBudgetExhausted
  MetadataBudgetExhausted
  CandidateBudgetExhausted
  ResultBudgetExhausted
  DepthBudgetExhausted
  DeadlineExceeded
  Cancelled
  Superseded
```

The implementation can use a non-atomic tracker inside one blocking traversal and shared atomics only where work is genuinely parallel. It SHALL expose one `try_consume_*`/checkpoint API so providers do not implement inconsistent off-by-one logic.

Budget is consumed **before** an operation that would exceed the limit:

- before opening/enumerating a directory;
- for each directory entry inspected, even if hidden, ignored, unreadable, or non-matching;
- before metadata/canonicalization/stat-like work;
- before opening a file;
- before each read chunk using the requested chunk size;
- before retaining a candidate/result;
- before descending past a depth.

The returned snapshot includes consumed counts and limits but no sensitive paths/content.

Current caps should seed initial defaults where applicable, for example existing path-entry/candidate caps, content file-size cap, and result cap. New aggregate limits must be chosen from measured current workloads and remain finite. Exact defaults live in code/config and tests, not in the normative spec.

### 4. Coverage and reason-code contract

Preserve the existing high-level coverage shape where possible:

```text
WorkspaceSearchCoverage:
  state: Complete | Partial | Unavailable
  reason_code?: stable enum/string
  budget?: bounded counter summary
```

Standard reason codes include at least:

```text
cancelled
superseded
inspection_busy
entry_budget_exhausted
directory_budget_exhausted
file_budget_exhausted
byte_budget_exhausted
metadata_budget_exhausted
candidate_budget_exhausted
result_budget_exhausted
depth_budget_exhausted
deadline_exceeded
unreadable_entries
provider_unavailable
provider_failed
invalid_cursor
stale_cursor
```

Semantics:

- `Complete`: traversal under selected ignore policy exhausted with no omitted eligible work.
- `Partial`: some valid output exists or traversal began, but work was omitted/stopped.
- `Unavailable`: inspection could not meaningfully start or no trustworthy result set can be returned.

One primary reason code is returned for stable UI behavior; a bounded structural summary MAY include secondary skipped/error counts. Raw OS/provider messages remain redacted diagnostics, not public reason codes.

An empty `Complete` result means no matches. An empty `Partial/Unavailable` result does not.

### 5. Global and per-workspace admission/backpressure

Introduce an application/runtime component:

```text
WorkspaceInspectionAdmission:
  global active limit
  per-workspace active limit
  bounded waiting queue or finite admission deadline
  operation class (directory/path/content/document/remote)
```

Admission is acquired before a blocking task or remote provider starts. The permit stays alive until the actual worker exits, including after caller cancellation.

Policy:

- same `search_id` supersession cancels prior work immediately;
- independent requests respect global and per-workspace caps;
- when waiting capacity is exhausted or its finite deadline passes, return `Unavailable/inspection_busy` without launching hidden work;
- no busy loop and no unbounded channel/queue;
- small direct directory navigation may have a separate reserved class only if current UX evidence requires it, but remains finite.

Bootstrap assembles this component. Commands do not acquire semaphores directly.

### 6. Shared recursive ignore policy

Create one application policy/value object and infrastructure matcher for recursive discovery:

```text
WorkspaceIgnorePolicy:
  product defaults
  repository .gitignore rules
  repository .ignore rules
  current request include/exclude overrides where already supported
  operation mode
```

Default recursive dependency/generated exclusions include current repository conventions such as:

```text
.git
node_modules
target
dist
build
coverage
.next
.nuxt
vendor
__pycache__
.pytest_cache
```

The implementation must reconcile these defaults with existing path-search exclusions rather than layering contradictory lists. Repository ignore syntax and negation are handled by one reviewed matcher implementation.

Operation modes:

- **RecursiveDiscovery/Search:** applies repository ignores and default generated/dependency exclusions.
- **DirectNavigation/ExplicitRead:** does not hide a user-addressed path merely because it is ignored, but all existing root/safety/size checks still apply.

The response includes the effective policy identity/version where useful for cursor compatibility and diagnostics, not the full potentially sensitive rule set.

### 7. Streaming content search

Refactor content search into one bounded traversal/read pipeline:

```text
walk eligible entry
→ consume entry/metadata budgets
→ apply ignore/type/size checks
→ consume file-open budget
→ open/read bounded chunks
→ binary/text detection
→ match and emit bounded result/snippet
→ stop/check cancellation and budgets frequently
```

It SHALL NOT first collect all candidate paths for the workspace. The traversal keeps only:

- the directory walk stack/iterator within depth and directory budgets;
- one current file/chunk;
- bounded result/snippet state;
- bounded error/coverage counters.

File growth and partial reads are reported through existing file-size/partial semantics; handle-based TOCTOU hardening remains a separate change.

Cancellation/deadline checkpoints occur at least before each directory, each entry batch, each file open, each read chunk, each match/result append, and before serialization/return.

### 8. Bounded-memory path search

Path search may need to visit many entries to find globally ordered matches without an index, but it SHALL not retain every candidate and sort the full vector.

Use a bounded selection structure appropriate to current ordering, for example a max-heap of the best `candidate_limit` items or a bounded top-K collector. It retains at most the configured candidate/result window plus fixed traversal state.

All visited entries consume the entry budget even if they do not match. Candidate retention consumes the candidate budget. If the traversal stops early, coverage is Partial with the precise reason.

Stable tie-breaking uses the current normalized sort key plus a deterministic relative-path/file identity fallback. This change does not alter search syntax or Unicode matching semantics.

### 9. Bounded-memory immediate directory pagination

Without a persistent index, finding the next sorted page can require scanning all immediate eligible entries. The adapter MAY perform that scan, but retains only the best `limit + 1` entries after the cursor using a bounded selection structure. Memory is therefore `O(limit)`, not `O(directory_entries)`.

Version the cursor:

```text
DirectoryCursorV2:
  version
  normalized workspace/directory identity
  order mode
  last sort key
  deterministic tie-break key
  directory fingerprint/generation
  ignore/navigation policy identity
```

The fingerprint uses the strongest inexpensive current-platform evidence available (for example directory metadata identity + modification generation). It is a compatibility detector, not a security boundary or perfect snapshot.

- malformed/wrong-directory/wrong-order cursors return `invalid_cursor`;
- detectable directory-generation mismatch returns `stale_cursor`;
- the frontend restarts pagination and replaces, rather than appends to, the prior page set;
- existing `truncated`/`has_more` continues to mean another page may exist;
- a separate coverage field explains whether the scan itself was complete.

Direct navigation listing does not apply recursive ignore defaults unless the existing product contract explicitly requests filtered listing.

### 10. Bounded document discovery

Document discovery uses the shared recursive ignore policy and budget tracker. It does not intentionally enter dependency/generated directories by default. Existing tests that encode traversal into `node_modules` or equivalent are replaced with:

- default exclusion tests;
- repository-negation/include override tests where supported;
- explicit direct navigation/read tests proving ignored content remains accessible when requested;
- partial coverage tests when budgets or unreadable entries prevent complete discovery.

Document metadata/snippet extraction follows the same file-open and byte budgets as content search or a stricter operation-specific profile.

### 11. Remote provider parity

Remote inspection adapters receive the same immutable request generation, budget profile, ignore-policy inputs, and cancellation token semantics. Because local counters cannot directly observe every remote syscall, the remote protocol/command SHALL enforce equivalent provider-side limits where possible and map returned structural counts.

On cancellation, supersession, or deadline:

- signal/close the remote command/channel using a bounded provider cancellation path;
- retain admission until the provider worker exits or is handed to an existing bounded cleanup owner;
- classify the result with the same public reason code;
- discard late output for a stale generation.

Remote implementation may use ripgrep/find/provider-specific flags, but raw shell command text and remote paths are not exposed in public errors/logs.

### 12. Frontend generation and result reconciliation

The frontend service request/result includes or internally tracks `search_id + generation`.

- A component/hook applies a result only if it belongs to the latest requested generation for that view.
- Cancel and supersede are distinct localized states where user experience benefits.
- Coverage reason and bounded budget summary are visible in an unobtrusive result notice; empty partial results are not rendered as definitive “no matches.”
- Stale directory cursor causes a controlled restart and replacement of accumulated pages.
- Tauri and Web/mock adapters return the same reason/state vocabulary.
- Web/mock uses deterministic fixtures and counters and MUST NOT claim that ignored files, bytes, or native providers were truly scanned.

### 13. Deterministic performance and correctness tests

Performance gates use instrumented fakes/temporary trees and assert:

- retained candidate count never exceeds configured bound;
- content search never holds a full candidate list;
- directory page retained entries are at most `limit + 1` plus fixed overhead;
- traversal stops at exact entry/directory/file/byte/metadata/depth/result limits;
- cancellation is observed within a bounded number of checkpoints/chunks, not a shared-CI millisecond threshold;
- active blocking/remote workers never exceed admission limits;
- same-id supersession cannot orphan the new registration;
- abort/drop eventually releases registry and admission after worker exit.

A small end-to-end latency smoke test may remain, but correctness does not depend on unstable absolute timings.

## Failure Semantics

Public errors/results use stable codes. Examples:

```text
workspace_inspection_busy
workspace_search_cancelled
workspace_search_superseded
workspace_search_partial
workspace_search_deadline_exceeded
workspace_search_budget_exhausted
workspace_search_provider_unavailable
workspace_directory_cursor_invalid
workspace_directory_cursor_stale
```

Specific coverage `reason_code` identifies the dimension. Native error detail is sent only through the unified redacted logging path.

## Migration Plan

No database migration is required.

Implementation sequence:

1. Add generation-safe registration, cancellation token, coverage/reason types, budget tracker, admission port, and fakes behind existing behavior.
2. Move local path/content/document providers to the common execution context.
3. Replace full candidate retention with streaming/top-K implementations.
4. Add cursor V2 and adapter/frontend compatibility.
5. Add shared ignore policy and update document-discovery behavior/tests.
6. Bring remote and Web/mock adapters to parity.
7. Remove explicit id-only `finish`, duplicated limit accounting, and unbounded admission paths after full tests pass.

## Risks / Trade-offs

- New actual-work budgets can return partial results where the old implementation eventually completed after excessive work. This is intentional and must be visible to users.
- Top-K avoids memory blow-up but still scans up to the entry/deadline budget without a persistent index. A future index is the path to sublinear scans.
- Ignore defaults improve common repositories but can omit deliberately vendored content from recursive discovery. Explicit navigation and request overrides preserve access.
- Cursor fingerprints cannot guarantee an immutable snapshot on all filesystems. Typed stale detection and complete/partial semantics are safer than silent page mixing.
- Admission may reject bursts as busy instead of allowing the blocking pool to saturate. Frontend debounce/supersession reduces user-visible impact.

## Open Questions

None. Concrete default budget values and file/type names are chosen during implementation from current repository constants and measured fixtures, but every value MUST remain finite, testable, and within the requirements above.
