## Context

See proposal.md — Why. Design-relevant current state:

- The terminal producer emits four spans from one shared attribute set containing only `vanehub.stage` and `vanehub.agent.id`. The classifier reads `vanehub.span.kind` first, then a fixed table of conventional attributes, and deliberately never reads span names. Neither emitted attribute appears in that table, so all four spans classify as unknown.
- The classifier's unclassified token is `unknown`, in the DTO, in the frontend's `kind === "unknown"` branch, and in the `traces.kind.*` translation keys. The spec says `other`. Nothing in the product emits or reads `other`.
- Run and span upserts resolve conflicts with `status = excluded.status` while carrying `ended_at` and `error_classification` forward through `COALESCE`. The separate finish path already guards with `status IN ('accepted', 'running')`. The two write paths therefore disagree about whether a terminal status is final.
- `timeline()` issues one query per run, one per run's links, one for all spans, **one per span for its links**, and one for events with a hard `LIMIT 5000`, all on a pooled connection with no transaction. `list_runs()` in the same file already implements keyset pagination with an opaque base64 cursor.
- The Traces panel puts a monotonically increasing refresh counter into both React Query keys. A refresh is therefore a different query with no data, which empties the run list, resets the selection through a correction effect, and unmounts the viewport that owns zoom, filter, and selection state.
- Duration absence is the sole input to "still running" in both bar geometry and the accessible label. The native projection returns absence for four distinct reasons, only one of which is "still running".
- Creation submits through an operation id and polls it. The submit guard checks `saveSshConnection` without checking `workspaceMode`; the enable guard checks it only inside the remote branch. Inspection results are assigned without any attribution to the request that produced them. The native completion path discards the result of recording operation completion. The frontend marks the operation handled before the canonical session read that follows it.

Constraint that shapes several decisions: this branch must not add a SQLite migration. Migration numbers are allocated across concurrent branches and every worktree on this machine shares one database file, so a speculative migration here is likely to collide. Every change below is therefore a write-path, query-shape, or projection change over the existing schema.

## Goals / Non-Goals

**Goals:**

- One validation result governs both the enablement and the execution of session creation.
- An inspection result can only be applied to the request that asked for it.
- A persisted session's operation completion is either durable or recoverable, and reconciliation is distinguishable from resubmission.
- A span's kind is asserted by its producer; a span's duration quality is reported separately from its lifecycle.
- A live refresh is an invalidation of existing data, not a new query identity.
- Status transitions are monotonic on every write path, using one shared definition of "terminal".
- A bounded event response states its own boundedness and offers a way past it.

**Non-Goals:**

- T01 (trace coverage for pre-launch startup failures), T07–T12 (N+1 batching, read snapshots, row-order unification, ARIA identity, depth limits, subscription health, critical-path naming), and C05 (structured error preservation) are out of scope for this change. They are real and separately tracked; mixing them in would make this change's regression surface unreviewable.
- No SQLite migration, no schema change, and no new Tauri command.
- No change to the four-layer terminal span topology, to canonical timestamps, or to what the CLI itself is able to observe. Declaring a tool span's kind does not make its interior visible; it stays `opaque`.
- No attempt to define a true causal critical path. This change does not touch that derivation.

## Decisions

### Reconcile the unclassified token to `unknown`, changing the spec rather than the code

The spec says `other`; the classifier, the DTO, the frontend branch, and the translation keys all say `unknown`. Renaming four implementation sites to satisfy a word would be a breaking DTO change that buys nothing, and `unknown` is the more truthful word: it says the producer did not assert a kind, where `other` implies a residual category the producer chose. The delta restates the requirement with `unknown`.

*Alternative considered:* change the implementation to `other`. Rejected — it is a visible contract change with no behavioral benefit, and it would make the token less accurate.

### Declare span kind per layer at the producer, not per span-name inference

`safe_attributes()` currently builds one attribute set shared by all four spans. Each layer instead gets its own set carrying `vanehub.span.kind`: `container` for the session and agent layers, `tool` for the terminal CLI layer, `process` for the exec layer. The classifier already trusts this attribute above all others, so no classifier change is needed.

The tool layer keeps `ExecutionFidelity::Opaque`. Kind and fidelity answer different questions — what this span is, versus whether its interior is observable — and the delta states that both must survive together. A test must assert the pair, because asserting only the kind would pass if fidelity regressed.

*Alternative considered:* infer from span name in the classifier. Rejected explicitly by the existing requirement and by the classifier's own documented rationale; name-based inference is what this classifier was built to replace.

The verification must run through the real path — producer attributes, `SafeAttributes` redaction, storage, query, DTO mapping — because the defect is precisely that each stage is individually correct while the composition yields `unknown`. A frontend fixture with a hand-written `kind` would prove nothing.

### Make status monotonic with a conditional expression, not `DO NOTHING`

The conflict branch must keep doing two things: refuse to regress lifecycle state, and still accept metadata that legitimately arrives late (`operation_id`, `provider_session_id`, `assistant_message_id`, links). `DO NOTHING` would satisfy the first and break the second.

The status assignment becomes conditional on the stored status while metadata assignment stays unconditional:

```sql
status = CASE WHEN <table>.status IN ('accepted', 'running')
              THEN excluded.status
              ELSE <table>.status END
```

The membership test is the same one `update_terminal` already uses, so both write paths share one definition of "terminal". Extracting that list into a single named constant used by both is part of the work; two copies of a correctness-critical predicate is how they drift.

*Alternative considered:* a monotonic revision or sequence column. Rejected for this change — it needs a migration, and this branch must not add one. It remains the right answer if out-of-order transitions later need full ordering rather than terminal protection.

### Report event coverage and page events with the cursor pattern already in the file

`list_runs()` already implements opaque base64 keyset pagination over `(started_at, run_id)`. Events get the same treatment over their existing `(timestamp, span_id, sequence)` ordering, so the ordering, the index usage, and the cursor encoding are all unchanged patterns rather than new ones.

The timeline response gains coverage metadata stating whether events were truncated and how to continue. Deliberately *not* "return the last 5000 instead": the failure this addresses is a consumer that cannot tell a complete record from a truncated one, and silently changing which 5000 are returned leaves that consumer exactly as misinformed while making the newest events look authoritative.

Both the Tauri adapter and the Web/mock adapter must return the coverage shape. The Web adapter reports complete coverage for its bounded fixtures — but it must report it explicitly, because a panel that treats "no coverage field" as "complete" would regress silently against the native runtime the moment the field is absent.

### Derive duration semantics from status plus duration, adding no DTO field

Lifecycle and measurement quality are already two separate fields: `status` carries lifecycle, and the absence of `completedDurationMs` carries "not measurable". The defect is that two rendering sites consult only the second. So `placeSpanBar` and `spanAccessibleLabel` take status into account, and the three cases separate cleanly:

| status | duration | presentation |
| --- | --- | --- |
| non-terminal | absent | running — open-ended bar, "still running" |
| terminal | absent | ended, duration unknown — bounded bar, explicitly unknown |
| any | present | measured |

No new DTO field is introduced, so no adapter contract changes for this part. `startOffsetMs` absence keeps its existing, already-correct `unplaceable` handling — that path is not part of this defect and must not be disturbed.

*Alternative considered:* a native `durationQuality` enum. Rejected as redundant — it would encode information the response already carries, and a second source for one fact is a future disagreement.

### Key queries by data identity; refresh by invalidation

`refreshToken` and `runListToken` leave the query keys, which become `["execution-runs", sessionId]` and `["execution-timeline", runId]`. `useTraceLiveRefresh` stops returning counters and instead invalidates those keys after the same 400ms trailing window, which is unchanged and still correct — the coalescing was never the problem.

This alone fixes the cascade, because React Query keeps a key's data across an invalidation: pages survive, the correction effect no longer sees an empty list, the viewport is not unmounted, and zoom, filters, and open detail survive with it.

Two consequences need explicit handling rather than falling out for free:

- **Following the newest run** must become an explicit mode. Today the panel re-selects the newest run as an accident of the correction effect emptying and refilling. With that gone, a reader on a historical run stays there, and a new run must announce itself without stealing the selection.
- **The comparison side** subscribes to its own run id. Today it reuses the selected run's token and therefore never refreshes on its own transitions.

*Alternative considered:* keep the token keys and add `placeholderData` to carry data across them. Rejected — it papers over the identity error with a visual, leaves one cache entry per refresh, and would need a guard to avoid showing one session's timeline under another's key.

### Give creation one validation result and attribute inspections to their request

Validation returns a single normalized result derived from the active `workspaceMode`. Both the enable guard and the submit guard read it, which makes the C01 class of defect unrepresentable rather than fixed: there is no second predicate to fall out of sync. SSH-save validation applies only when the mode is remote *and* saving is requested — matching what the connection-creation branch already does correctly.

Inspections carry a request identifier and their normalized path; a result is applied only if both still match current state, and the identifier advances when the dialog opens, closes, or switches workspace mode. This is logical cancellation, not transport cancellation — the IPC call is not aborted, its result is ignored. The dialog's open-effect must also reset the state it currently leaves behind (`inspection`, `worktreeEnabled`, `worktreeName`, `multiSeats`), which is what lets a reopened dialog show a stale Git capability for a cleared path.

### Separate creation states and mark handled only on delivery

Creation becomes an explicit state rather than two booleans: creating → operationSucceeded → resolvingSession → resolved, with recoverableError reachable from the read step. The operation is marked handled only after a session has been delivered, so a transient read failure retries the read instead of ending the flow. Retrying reads; it never resubmits — that distinction is the whole point, since resubmission creates a second session.

Natively, the completion write stops being discarded. A completion for an already-persisted session is idempotent, so a retry completes the existing operation rather than creating anything. Full transactional unification of session persistence and operation completion is not attempted here — they are separate stores, and forcing one transaction across them is a larger change than this correctness fix warrants; a recoverable, idempotent completion is the bounded version that removes the stranded-operation outcome.

## Risks / Trade-offs

- **The `CASE` expression silently does nothing if the status vocabulary drifts** → the terminal-status list becomes one shared constant referenced by both write paths, with a test that applies start-after-finish for every status value rather than one representative.
- **Removing tokens from query keys changes what `use-trace-live-refresh.test.ts` asserts** → that suite currently asserts counter increments, which is testing the mechanism rather than the behavior. It must be rewritten to assert invalidation and, more importantly, state survival, which is what the requirement actually says.
- **Losing accidental follow-the-newest-run behavior could read as a regression** → the explicit affordance and the new-run indicator ship in the same change as the key fix, never after it.
- **Adding coverage to the timeline DTO touches both adapters** → both are updated together, and the Web adapter states complete coverage explicitly rather than omitting the field.
- **Declaring span kinds changes what the existing producer test asserts** → that test currently asserts the exact attribute key list, so it fails as soon as a kind is added. That failure is the test doing its job; it is updated to assert kind and fidelity per layer through the real classification path.
- **The four terminal spans still share one start and one end timestamp** → unchanged here and out of scope, but it means the waterfall bars remain coincident. Duration-quality work must not be reported as having improved waterfall readability, because it has not.
- **These fixes are individually small and collectively broad** → each requirement gets a regression test that fails before its fix. The audited defects are all invisible to the 70 tests that pass today, so "the suite is green" is not evidence for any of them.

## Migration Plan

No schema migration, no data backfill, no new command. The status change is write-path only and applies to rows already stored. Event paging reuses existing ordering and adds no index. Rollback is a code revert; no persisted state depends on this change.

Rows that already reached the contradictory state this change prevents — non-terminal status with a terminal timestamp, from a start that overwrote a finish — are not repaired. They are unreachable by any current UI path that reads status without also reading the timestamp, a backfill would need a migration this branch must not add, and inventing a status for a row whose true outcome is unknown would be the same class of error this change exists to remove.
