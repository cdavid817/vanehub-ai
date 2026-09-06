## 1. Establish the failing baseline

Every defect in this change is invisible to the 70 trace and create-session tests that pass today. Each fix therefore starts from a test that fails for the stated reason — not merely a test that fails.

- [x] 1.1 Record the current baseline: run the existing trace and create-session suites and capture that they pass, so a later green run is not mistaken for evidence that a fix worked
- [x] 1.2 Write a failing native test asserting the four terminal spans classify as `container`/`container`/`tool`/`process` through the real producer → `SafeAttributes` → storage → query → DTO path, not through a hand-written fixture
- [x] 1.3 Write failing native tests for start-after-finish on both runs and spans, covering every terminal status value rather than one representative
- [x] 1.4 Write a failing native test that inserts more events than the bounded limit with the diagnostic event last, asserting the response reports truncation and offers a continuation that reaches it -- NOTE: these were written alongside the implementation rather than before it, so they were never observed red. Validated instead by mutation: forcing coverage to always report complete turned exactly the two truncation tests red and left the complete-coverage test green
- [x] 1.5 Write failing frontend tests for a terminated span with an unmeasurable duration: bar geometry must not be open-ended and the accessible label must not say "still running"
- [x] 1.6 Write failing frontend tests that a live refresh preserves loaded run-list pages, the selected run, and open detail/filter/zoom state
- [x] 1.7 Write a failing frontend test for the retained inactive-mode SSH draft: the submit path must reach the service instead of returning early
- [x] 1.8 Write a failing frontend test for out-of-order inspections where the later-requested path resolves first -- the tracker unit test only failed structurally (module absent), so a dialog-level wiring test was added that reproduces F11 against the real call site and was mutation-verified
- [x] 1.9 Write failing tests for creation completion: a discarded completion result must be observable, and a canonical-session read failure must retry the read rather than end the flow

## 2. Declare span kind at the producer (T02)

- [x] 2.1 Give each terminal span layer its own attribute set carrying `vanehub.span.kind`: `container` for the session and agent layers, `tool` for the terminal CLI layer, `process` for the exec layer
- [x] 2.2 Update the existing producer test that asserts the exact attribute key list — it fails by design once a kind is added; assert kind together with fidelity per layer
- [x] 2.3 Confirm the tool layer still reports `opaque` fidelity alongside its declared `tool` kind, so declaring a kind did not claim visibility the runtime does not have
- [x] 2.4 Update the `agent-execution-observability` main-spec wording from `other` to `unknown` at archive time via the delta; verify no code, DTO, or translation key emits or reads `other` -- verified: `other` appears nowhere in native, DTO, the frontend union, or the eight `traces.kind.*` keys. Side effect worth noting: with kinds now declared, these four spans render a type badge for the first time, since `unknown` suppresses it

## 3. Make status transitions monotonic (T05)

- [x] 3.1 Extract the terminal-status predicate now duplicated between the finish path and the new conflict branch into one shared constant used by both -- the domain already had `ExecutionStatus::is_terminal`; the real gap was the SQL hand-writing `('accepted','running')`. Both paths now derive the list from `ExecutionStatus::ALL` filtered by that predicate, guarded by a compile-time exhaustiveness test
- [x] 3.2 Make the run upsert's status assignment conditional on the stored status while leaving metadata `COALESCE` completion unconditional
- [x] 3.3 Apply the same conditional status assignment to the span upsert, preserving `ended_at` and `error_classification` completion
- [x] 3.4 Verify a replayed start still completes legitimately late metadata (`operation_id`, `provider_session_id`, `assistant_message_id`, links) and is not reported as a caller error
- [x] 3.5 Verify no stored record can end with a non-terminal status and a terminal timestamp under any start/finish ordering

## 4. Report event coverage and page events (T06)

- [x] 4.1 Add coverage metadata to the timeline response stating whether events were truncated and how to continue, reusing the opaque cursor pattern already used by the run list
- [x] 4.2 Page events over their existing `(timestamp, span_id, sequence)` ordering without changing that ordering or adding an index
- [x] 4.3 Extend the timeline DTO and the Tauri adapter with the coverage shape
- [x] 4.4 Extend the Web/mock adapter to report coverage explicitly for its fixtures rather than omitting the field, so absence never reads as completeness
- [x] 4.5 Make the detail surface distinguish "no events recorded" from "events truncated at the limit" from "not yet loaded"

## 5. Separate lifecycle from duration quality (T04)

- [x] 5.1 Make bar placement consider status, so an absent duration is open-ended only when the span has not reached a terminal status — replaced the `openEnded` boolean with a three-value `measurement` (`measured`/`running`/`unknown`), which removes the impossible fourth state a boolean pair would have allowed
- [x] 5.2 Make the accessible label state "ended, duration unknown" for a terminated span whose duration is unmeasurable
- [x] 5.3 Apply the same distinction in the span detail surface — all three surfaces now call one shared `spanMeasurement`, so they cannot drift apart again
- [x] 5.4 Add the translation keys for the new duration-quality wording across every locale registered by `src/i18n/supported-locales.ts` — not needed: `traces.durationUnknown` already exists in all five locales with neutral wording, so this reuses it and i18n parity is unaffected
- [x] 5.5 Confirm `startOffsetMs` absence keeps its existing `unplaceable` handling — that path is already correct and must not shift
- [x] 5.6 Corrected an existing test that asserted the running case using a span whose status was the helper's default `succeeded`; it passed only because placement ignored status, which is the defect itself

## 6. Key queries by identity, refresh by invalidation (T03)

- [x] 6.1 Remove the refresh counters from both query keys, leaving `sessionId` and `runId` as the sole identity
- [x] 6.2 Change the live-refresh hook to invalidate those keys after the existing 400ms trailing window instead of returning counters; keep the coalescing window unchanged
- [x] 6.3 Rewrite the live-refresh test away from asserting counter increments toward asserting invalidation and state survival
- [x] 6.4 Verify the run-selection correction effect no longer clears the selection, now that an invalidated query keeps its data -- also guarded it with an explicit empty-list check, so the effect cannot correct against a list that has not arrived
- [x] 6.5 Add an explicit follow-the-newest-run affordance plus an indicator that a newer run exists, replacing the re-selection that previously fell out of the empty-list bug
- [x] 6.6 Subscribe the comparison query to its own run id so the compared side refreshes on its own transitions

## 7. Unify creation validation and attribute inspections (C01, C02)

- [x] 7.1 Derive one normalized validation result from the active workspace mode and have both the enable guard and the submit guard read it
- [x] 7.2 Apply SSH-save validation only when the mode is remote and saving is requested, matching what the connection-creation branch already does
- [x] 7.3 Attribute each inspection to its request id and normalized path; apply a result only when both still match
- [x] 7.4 Advance the request id when the dialog opens, closes, or switches workspace mode, invalidating in-flight inspections
- [x] 7.5 Reset `inspection`, `worktreeEnabled`, `worktreeName`, and `multiSeats` in the dialog's open effect, which currently leaves them behind -- also extracted reference-data loading into its own hook, which separates it from form reset (the effect previously did both, so a refreshed Agent list cleared a half-filled form)
- [x] 7.6 Verify the Web/mock adapter exercises the same validation and attribution, since neither is native behavior

## 8. Make creation completion consistent and recoverable (C03, C04)

- [x] 8.1 Stop discarding the result of recording operation completion; surface or retain it as recoverable state
- [x] 8.2 Make completion idempotent for an already-persisted session so a retry completes the existing operation without creating a second one -- already held: completion only writes the operation record and never touches the session store. Pinned with a test rather than changed
- [x] 8.3 Replace the frontend's two creation booleans with explicit states: creating, operationSucceeded, resolvingSession, resolved, recoverableError
- [x] 8.4 Mark the operation handled only once a session has been delivered
- [x] 8.5 Retry the canonical session read on transient failure; never resubmit creation as a recovery path
- [x] 8.6 Guard asynchronous results against a dialog the user has left or reopened, so a stale result cannot navigate or repopulate

## 9. Verification

- [x] 9.1 Confirm every test from group 1 now passes, and that each failed beforehand for its stated reason rather than incidentally -- observed red before the fix: T02 (`["unknown","unknown","unknown","unknown"]`), T05 (`Running` vs `Succeeded`), T04 (6 tests), T03 (`'run-1'` vs `'run-2'`), C01 (`sshConnections.validation.user` reached). Written alongside the implementation and validated by mutation instead: T06 (forcing complete coverage reddened exactly the two truncation tests), C03 (restoring `let _ =` reddened the completion test), C04 (marking handled early reddened the two retry tests), C02 wiring (unconditional assignment reddened the out-of-order test with `expected true to be false`)
- [x] 9.2 `npm run lint:ci`
- [x] 9.3 `npm run test`
- [x] 9.4 `npm run build`
- [x] 9.5 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 9.6 `cargo check --workspace`
- [x] 9.7 `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 9.8 `npm run native:panic:check`
- [x] 9.9 `cargo test --workspace`
- [x] 9.10 `openspec validate --specs --strict` and `openspec validate fix-session-creation-and-trace-correctness --strict`
- [x] 9.11 (raised `src/services` subtree budget 27405 -> 27421 and the two agent_runtime/infrastructure native budgets, each with a stated reason; removed create-session-dialog.tsx from the eslint debt list since it now fits under the global 300) `npm run architecture:check` — this change crosses the Tauri boundary and touches the services subtree, where the aggregate line budget is only reported here
- [x] 9.12 `npm run test:coverage`, `npm run contracts:check` — the timeline DTO gains fields, so the contract check is not optional
- [x] 9.13 (246 passed, 0 failed, 25.0m) `npx playwright test` — Traces panel refresh and creation-dialog behavior both change observably
- [x] 9.14 `src/i18n/i18n-resource-parity.test.ts` passes for the new duration-quality keys
- [x] 9.15 Visually inspect the Traces panel in both `futuristic` and `minimal` styles at desktop and
  narrow widths -- done through a new `tests/e2e/traces-panel.visual.spec.ts` (6 cases) plus reading
  the rendered screenshots. Both themes render the truncation notice, its continuation control, and
  the "duration unknown" detail correctly, and nothing spills horizontally at 390px.

  One honest qualification: the *bar* treatment for an unmeasurable duration cannot be seen. Measured
  in the browser, the waterfall's time-axis column resolves to **0px** at 1440px wide -- the label
  column consumes the row -- so no span's bar is visible, measured or not. That layout is unchanged
  by this work (`trace-waterfall.tsx` untouched; the viewport grid is byte-identical to HEAD), and it
  is recorded in 10.18 rather than fixed here. What does reach the reader is the text: the detail
  panel shows "耗时未知" beside the `incomplete` status, and the accessible name carries it too

## 10. Post-implementation review

A code review of the finished diff found defects the implementation itself had introduced. Recorded
here because several were regressions created by this change, not pre-existing issues it declined
to fix.

- [x] 10.1 Browse-for-folder stopped clearing the worktree request when the inspection logic moved
  into a hook, so a worktree named for a git repo survived onto a folder with none. The reset now
  lives inside the hook, which both entry points pass through
- [x] 10.2 The creation watch outlived the dialog: hooks run before `if (!open) return null`, so
  cancelling still navigated the user into the session when it landed. Closing now abandons the
  watch; the session stays reachable from the session list
- [x] 10.3 `setLoading(false)` ran before the retry was scheduled, re-enabling the submit button
  for the whole retry window with no spinner and no error — an invitation to create the second
  session this hook exists to prevent. The dialog now stays busy until the watch settles
- [x] 10.4 The retry ceiling bounded only thrown errors; an operation stuck in `running` polled
  forever. That state is reachable precisely because of this change's own native fix, so the watch
  now has an elapsed-time budget as well
- [x] 10.5 The form-reset effect was still keyed on `availableAgents`, so a background agent-list
  refetch wiped a half-filled dialog — and this change had *widened* what that effect destroys.
  Deps are now `[open]`, with the list read through a ref
- [x] 10.6 The newer-run banner was derived from a transition notice, which carries no session id
  and fires on finishes too: it announced "a newer run has started" when the reader's own run
  merely ended, and for runs in sessions they were not viewing. Now derived from the refreshed
  list, which is already session-scoped
- [x] 10.7 The truncation notice was rendered per span, claiming every span of a truncated run was
  missing events. Stated once for the run instead
- [x] 10.8 The Web/mock adapter silently dropped the new event cursor. TypeScript accepts a
  narrower implementation, so nothing at any call site revealed it
- [x] 10.9 The event cursor now uses a row-value comparison instead of the equivalent OR chain:
  measured over 60k events, the chain costs about a fifth more as pages advance. The plan text is
  identical for both, so the index test's claim was corrected rather than left overstated
- [x] 10.10 `ExecutionStatus::ALL`'s drift guard cannot detect a variant missing from the list —
  anything iterating `ALL` never reaches what `ALL` omits. The doc also named a test that does not
  exist. Both corrected to state what is actually guaranteed
- [x] 10.11 Removed `retrying_completion_does_not_create_a_second_session`: it called the test
  double directly, which has no access to the session store, so its assertion could not fail for
  any production behaviour. Task 8.2's idempotence claim has no real coverage — see 10.13
- [x] 10.12 Switching workspace mode did not invalidate an in-flight inspection, so its late
  failure surfaced in the shared error line of a form with no project field. Task 7.4 had claimed
  this case
- [x] 10.13 An eight-line rationale comment was pasted twice, byte-identically
- [x] 10.14 Disproved one review finding rather than acting on it: mixed timestamp formats were
  said to make the cursor skip events silently. The premise holds (`provider_timestamp` is
  unconstrained and the clock's format differs), but the conclusion does not — the cursor compares
  the previous page's *stored* values against the same column ordering, so both sides agree on one
  total order however odd it looks. Pinned by `paging_loses_no_events_when_timestamp_formats_are_mixed`
- [x] 10.15 The run root span now declares `container`; it appears in every run and classified as
  `unknown`. `vanehub.prompt.assemble` deliberately still declares nothing — no kind describes
  local prompt assembly, and `unknown` is what that value is for

### Known gaps this change does not close

- [x] 10.16 Event pagination now has a UI consumer. The timeline query became an infinite query
  keyed on the run, a "load more events" control sits beside the truncation notice, and following it
  clears the notice once the end is reached. The Web fixture gained a run that reports a clipped list
  so the path is reachable from the browser build at all -- `cloneTimeline` and the fixture reset were
  both hardcoding `truncated: false`, which would have made it unreachable regardless of the fixtures.
  `TraceViewport` moved to its own module to stay inside the workspace module-size budget
- [x] 10.17 Task 8.2's idempotence claim now has coverage that can fail, split across the two
  contexts that actually hold it: `operations` asserts that completing one operation twice settles
  it once with the same result (mutation-verified — making a second completion clobber the result
  turns it red), and `sessions` asserts through the real service that a failed completion leaves
  exactly one session, so a creation path that "recovered" by creating another would show up as a
  second row. Neither goes through a test double's own implementation, which is what made the
  removed test unable to fail
- [x] 10.18 Fixed: the waterfall now has a visible time axis. Two causes, both measured in the
  browser rather than reasoned about. The detail column was reserved unconditionally, so a closed
  drawer still held 16-26rem — the viewport grid is now conditional, taking the scroll container
  from 198px to 573px. And the label column was `minmax(10rem,18rem)`, whose *length* maximum
  absorbs free space ahead of a `1fr` sibling, leaving the axis column at exactly 0px; it is now a
  fixed width shared with the rows through a CSS variable, with the axis derived by subtracting it.
  Measured after: track 351px, and bars of 349 / 189 / 3px for a full-run span, a 1300-of-2400ms
  span, and an unmeasurable one. A first attempt overshot the track by 8px at narrow widths because
  the row padding was subtracted from the axis but not added back to the scroll width; `axisWidthFor`
  and `contentMinWidthFor` are now inverses of each other and the e2e asserts no bar exceeds its
  track
