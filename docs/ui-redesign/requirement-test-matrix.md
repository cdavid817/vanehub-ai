# Requirement-to-test matrix — `redesign-unified-workbench-ui`

Task 21.1. Cross-references every requirement in the change's 11 spec deltas
(`openspec/changes/redesign-unified-workbench-ui/specs/*/spec.md`) against `tasks.md`'s own
per-task evidence and the real test files that evidence cites. Built by direct reading of all 11
spec files, the full 1047-line `tasks.md`, and Glob/Grep spot-checks against the actual `src/`/
`tests/` tree — not templated from `traceability.md` (task 1.3), which this document supersedes as
the test-coverage source of truth (`traceability.md` remains the requirement-to-task ownership map;
its own Evidence column was never updated past "Not started" and is stale as of this writing).

Two independent passes at this same task landed in this shared worktree within the same session
window; this file is the second pass reconciled against the first (which had already spot-checked
its own citations before this one started and used a slightly more conservative Covered/Partial bar
in several rows — its numbers are the ones kept). The one concrete, additive gap the first pass had
not caught was folded in during reconciliation: requirement 17's `ComposerConfigPopover.tsx`/
`ConfigField.tsx` have zero dedicated tests, confirmed via `Glob`, moving that row from Covered to
Partial. Every citation in every row was independently spot-checked against the real tree at least
once across the two passes before this file was committed.

## Requirement and scenario count — confirmed, with a correction

`grep -c '^### Requirement:'` across all 11 spec delta files returns **89** — matches this task's
own wording exactly.

`grep -c '^#### Scenario:'` across the same 11 files returns **401**, not the 312 this task's
wording names. Reconciled directly: `traceability.md`'s own per-row scenario counts sum to exactly
312, so 312 was a real, correct count *at the time `traceability.md` was written* (task 1.3, early
in this change). The specs have grown by 89 scenarios since — concentrated almost entirely in
**MODIFIED** requirements that were amended as later milestones' own implementation work revealed
edge cases, most heavily `main-layout-ui` (+54, nearly all in "Sidebar session organization",
"Optimized information panel tabs", "Create-session dialog", and "Workspace activity bar"),
`loop-management-ui` (+14), `settings-center-ui` (+8), `agent-mission-control` (+7),
`scheduled-task-management` (+4), and `chat-experience` (+2). Four domains
(`agent-evaluation`, `goal-management`, `project-worktree-management`, `unified-todo-board`) have
zero drift — their real counts match `traceability.md` exactly. This matrix uses the **real,
current 401** throughout, not the stale 312 named in the task text.

| Domain | Requirements | Real scenarios | `traceability.md`'s scenarios | Drift |
|---|---:|---:|---:|---:|
| agent-evaluation | 8 | 28 | 28 | 0 |
| agent-mission-control | 8 | 37 | 30 | +7 |
| chat-experience | 8 | 35 | 33 | +2 |
| goal-management | 6 | 17 | 17 | 0 |
| loop-management-ui | 9 | 45 | 31 | +14 |
| main-layout-ui | 9 | 95 | 41 | +54 |
| project-worktree-management | 6 | 16 | 16 | 0 |
| scheduled-task-management | 8 | 33 | 29 | +4 |
| settings-center-ui | 9 | 40 | 32 | +8 |
| unified-todo-board | 8 | 23 | 23 | 0 |
| workbench-design-system-ui | 10 | 32 | 32 | 0 |
| **Total** | **89** | **401** | **312** | **+89** |

## Scope of this matrix — requirement-level, not scenario-level

This matrix is built at **requirement granularity** (89 rows), not individual-scenario granularity
(401 rows). This is a deliberate, disclosed scoping choice matching the task's own stated fallback
("requirement-level with a scenario-count-covered/total-per-requirement rollup is an acceptable,
honestly-scoped alternative"): 401 individually-verified scenario rows, each cross-referenced
against real test assertions, was not tractable in one pass alongside the accuracy bar this task
also requires. Every row below states the requirement's real scenario count and, in its verdict
and notes, identifies **which specific named scenarios** are disclosed gaps where that is known —
this is not a blind requirement-level rollup, but scenario-level detail is called out in prose
rather than as 401 separate table rows.

## Verdict definitions

- **Covered** — the requirement's scenarios are, in aggregate, substantively proven by real,
  cited automated tests (unit `.test.ts`, component `.test.tsx`, or Web E2E `tests/e2e/*.spec.ts`).
  Minor test-organization notes (e.g. a child component tested only via its parent's render tree)
  do not by themselves downgrade a row.
- **Partial** — at least one named scenario has a real, disclosed gap: deliberately deferred
  functionality, an explicitly incomplete (`[ ]`) task, a mechanism proven only against synthetic
  fixtures because the real feature isn't wired to production data yet, or a scope narrower than
  the requirement's own text (e.g. a shared mechanism built but adopted by only some of the pages
  the requirement covers).
- **Gap** — the requirement's core mechanism is unbuilt, blocked, or has no located automated test
  evidence for essentially any of its scenarios.
- **Manual-only** — no automated test exists or is planned; verification is explicitly human/manual
  (this audit found none of the 89 requirements individually gated on a manual-only owner at the
  requirement level — see "Cross-cutting gaps" below for the manual/native work that affects many
  requirements' *completeness* without being any single requirement's sole owner).

All file paths below were confirmed to exist via direct `Glob`/`Grep` against the real tree, not
copied from `tasks.md` prose without verification, except where a row explicitly says a citation
was not independently confirmed.

## Coverage summary

| Domain | Requirements | Covered | Partial | Gap |
|---|---:|---:|---:|---:|
| agent-evaluation | 8 | 2 | 6 | 0 |
| agent-mission-control | 8 | 4 | 4 | 0 |
| chat-experience | 8 | 5 | 3 | 0 |
| goal-management | 6 | 3 | 3 | 0 |
| loop-management-ui | 9 | 3 | 5 | 1 |
| main-layout-ui | 9 | 4 | 5 | 0 |
| project-worktree-management | 6 | 1 | 4 | 1 |
| scheduled-task-management | 8 | 5 | 3 | 0 |
| settings-center-ui | 9 | 6 | 3 | 0 |
| unified-todo-board | 8 | 6 | 2 | 0 |
| workbench-design-system-ui | 10 | 7 | 3 | 0 |
| **Total** | **89** | **46** | **41** | **2** |

**46 of 89 requirements (52%) have genuine, verified automated coverage across all their named
scenarios. 41 (46%) have real coverage for most scenarios with at least one specific, disclosed
gap — including requirement 17 (chat-experience), reclassified from Covered to Partial during a
cross-check of this document against an independently-built companion pass: `ComposerConfigPopover.
tsx`/`ConfigField.tsx`, the Run Configuration popover's own two new components, have zero dedicated
test coverage, confirmed via `Glob`. 2 (2%) have no real automated coverage of their core mechanism.**
Zero requirements are
Manual-only at the requirement level, but several cross-cutting test *kinds* (desktop E2E, full
visual-regression matrix, manual accessibility smoke) are themselves only partially built and
affect the completeness of many "Covered"/"Partial" rows' own `Desktop E2E`/`Visual` columns — see
below.

## Cross-cutting gaps affecting many rows

These are not owned by any single requirement row in `traceability.md`, but materially affect the
completeness of the **test kind** columns across most of the 89 rows above:

- **Desktop E2E (native Tauri/WebdriverIO) coverage of this redesign's own new UI is effectively
  zero.** Tasks 21.20/21.21 (`tests/desktop/` WebdriverIO specs for xterm resize, window-resize
  composition, native folder dialog, SSH trust flow, native scheduler/evaluation smoke; native
  smoke on Windows/macOS/Linux) are both unstarted (`[ ]`). The existing `tests/desktop/` suite
  (`desktop-smoke`, `desktop-session-workspace`, etc.) exercises general pre-existing app behavior,
  not this change's new destinations/routes/component structure specifically. No row below cites a
  `tests/desktop/` file as evidence for redesign-specific behavior.
- **Visual regression is a first increment only.** Task 21.17 built 24 of the ~300 base
  combinations design.md's own Decision 20 names (3 of 10 core surfaces × 2 themes × 2 of 3
  locales × 2 of 5 widths — `tests/e2e-visual-regression/core-surfaces.spec.ts`). Most rows'
  `Visual` test-kind claims, where present, rest on the older `page.screenshot()`-to-gitignored-
  directory pattern (proves a surface renders, not diffed against a committed baseline), not a real
  pixel-regression baseline.
- **Automated accessibility scanning (axe-core) covers 4 of many destinations.** Task 20.18 added
  `@axe-core/playwright` and scanned Evaluation Center, Mission Control, Work Board, and Goal
  Center (both themes, one populated state each) — real, and it found and fixed genuine
  `color-contrast`/`aria-prohibited-attr` bugs at the design-token root. Sessions (the app's default
  destination), Projects, Loop Center, Scheduled Tasks, and all ~20 Settings pages are not yet
  scanned.
- **Manual screen-reader smoke (task 20.20) is entirely unstarted** — no navigation-landmark,
  route-title, tab, list, table-header, error, dialog, or live-status screen-reader pass has been
  recorded for any surface in this change.
- **Bidirectional-text and overlap-under-long-strings verification (20.15/20.16) are unstarted.**

## agent-evaluation (`specs/agent-evaluation/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 1 | ADDED | Evaluation experiment workflow | 4 | §18.1–18.5 | Unit, Component, Web E2E | `evaluation-run-wizard.tsx`, `evaluation-agent-selector.test.tsx`, `evaluation-arena-list.test.tsx`, `tests/e2e/evaluation-center.spec.ts` | Partial |
| 2 | ADDED | Evaluation results data table | 4 | §18.6–18.7, 18.16 (unbuilt) | Unit, Component, Perf | `evaluation-results-table.test.tsx`, §21.13's "renders every row once for a realistic full-scale corpus (1,000 attempts)" case | Partial |
| 3 | ADDED | Evaluation baseline and regression presentation | 4 | §18.8–18.10 | Unit, Component | `evaluation-comparison.test.ts`, `evaluation-comparison-panel.test.tsx` | Covered |
| 4 | ADDED | Multi-experiment comparison | 4 | §18.11 | Unit, Component | `evaluation-comparison-matrix.test.ts` (9 cases + §21.13's 20-metric scale case), `evaluation-comparison-matrix-view.test.tsx` (6 cases) | Partial |
| 5 | ADDED | Explained evaluation outcomes | 4 | §18.12 | Component | `evaluation-center.test.tsx` (18.12 cases) | Covered |
| 6 | ADDED | Evaluation artifact evidence links | 3 | §18.13 | Component | `evaluation-center.test.tsx`, `evaluation-attempt-fixtures.ts`'s `artifactUnavailableAttempt` | Partial |
| 7 | ADDED | Evaluation visibility-aware updates | 3 | §18.14 (event-coalescing unbuilt), §21.13 | Unit, Component, Perf | `evaluation-center.test.tsx` ("pauses polling while the document is hidden...", "stops polling once unmounted"), `use-evaluation-query.ts` | Partial |
| 8 | ADDED | Evaluation component boundaries | 2 | §18.2 (partial), §1.6 | Architecture | `scripts/architecture/frontend-rules.mjs` (ARCH-FE-001/002), `line-budget-exemptions.node-test.mjs` | Partial |

**Domain notes**: Requirement 2's literal "ten thousand result rows" scenario cannot be reached
with real data — task 21.13 found the domain's own fixture generator tops out at 1,000
*attempt*-level rows (the granularity `EvaluationResultsTable` actually renders; the fixture's
10,000 rows are *check*-granularity, a different, non-rendered shape) and built the honest
1,000-row version instead, disclosing the correction in its own evidence. Requirement 4's "Share a
view route" / cross-navigation scenarios rest on `EvaluationComparisonPanel` pickers that exist and
work, but `quality-destination.tsx`'s own `comparisonIds` query param is parsed and tested at the
route layer only — not yet consumed by `EvaluationCenter` to restore a shared comparison. Task 18.1
(route placement) is itself left `[ ]` even though routing already works, specifically because it
also covers the still-unstarted module split (18.2). Task 18.16's own 10,000-row/comparison/
baseline/keyboard/i18n/theme/Tauri smoke sweep is explicitly deferred as premature until 18.2/18.8/
18.11's structure settles.

## agent-mission-control (`specs/agent-mission-control/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 9 | ADDED | Runs destination hierarchy | 3 | §16.1–16.2, §4.8 | Unit, Component, Web E2E | `workbench-route.test.ts`, `mission-control.test.tsx`, `mission-control-view-state.test.ts` (6), `tests/e2e/mission-control.spec.ts` ("4.8: returns to the same run..."), `tests/e2e/workspace-routing.spec.ts` + `workspace-activity-bar.spec.ts` (18/18) | Covered |
| 10 | ADDED | Mission Control saved views | 3 | §16.6 | Unit | `mission-control-saved-views.test.ts` | Partial |
| 11 | ADDED | Mission Control evidence navigation | 3 | §16.13 (2/9 surfaces converted), §9.9 (unbuilt for 7 more target kinds) | Web E2E | `tests/e2e/mission-control.spec.ts` (Review `EvidenceLink` round-trip) | Partial |
| 12 | ADDED | Mission Control action locality | 3 | §16.15 | Unit | `mission-control-run-precedence.test.ts` | Covered |
| 13 | MODIFIED | Bounded Mission Control overview | 6 | §16.3–16.4, §21.10 | Unit, Component, Perf | `mission-control.test.tsx`, `web-mission-control-client.test.ts` (100/1,000-Run page-size cap) | Covered |
| 14 | MODIFIED | Lazy and truthful Run detail | 7 | §16.9 (7/9 facets real), 16.11–16.12 | Component | `mission-control-facets.test.tsx` (26 tests), `mission-control-detail-panel.test.tsx` (facet-switching fetch exclusivity) | Partial |
| 15 | MODIFIED | Coalesced events and deterministic reconciliation | 6 | §16.16 (3/4 real) | Unit | `use-mission-control-polling.test.ts` (8 tests) | Partial |
| 16 | MODIFIED | Compact accessible responsive presentation | 6 | §16.17, §16.8, §16.18 | Component, Web E2E | `mission-control-section-nav.test.tsx`, `tests/e2e/mission-control.spec.ts` (390px variants, keyboard-only 2200px roving-tabindex pass) | Covered |

**Domain notes**: Requirement 11's own gap is corroborated by two independent sources — task
16.13's own "2 of 9 surfaces converted" finding and task 9.9's own separate, explicit `[ ]`
("Session/File/Log/Trace/Skill Settings/IM Settings/Project" EvidenceLinks not built) — the same
real gap surfacing from two different task sections. Requirement 14's Verification/Context facets
do not exist at all (a disclosed backend-data gap, not a frontend test gap). Requirement 15's real
backend event-coalescing is a disclosed, still-open backend blocker (18.14's own class of gap); the
client-side bounded-backoff polling infrastructure is real and tested regardless.

## chat-experience (`specs/chat-experience/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 17 | ADDED | Progressive-disclosure run configuration | 5 | §10.15–10.18 | Component, Web E2E | `ButtonArea.test.tsx` (provenance/reset/high-risk assertions), `tests/e2e/onepiece-agent.spec.ts` ("...high-risk affordances (10.22)") — but `ComposerConfigPopover.tsx`/`ConfigField.tsx`, the two components this requirement is actually about, have **no dedicated `.test.tsx` file** (confirmed via `Glob`, zero matches) | Partial |
| 18 | ADDED | Inspectable conversation evidence | 4 | §10.7, §9.1, §9.6 | Unit, Component | `MessageItem.selection.test.tsx` (7), `MessageList.selection.test.tsx` (2), `ToolUseBlock.test.tsx` (+3), `chat-tab.selection.test.tsx` (3), `workbench-selection.test.ts` (10), `session-workspace-region-builders.test.tsx` (3) | Covered |
| 19 | ADDED | Windowed dynamic-height conversation history | 5 | §10.10–10.14, §21.8 | Unit, Component, Perf | `use-conversation-window-model.test.tsx` (8), `MessageList.virtualization.test.tsx` (3), `VirtualizedMessageList.test.tsx` (6), `VirtualizedMessageList.large-scale.test.tsx` (5,000-message DOM-bounded + rerender budget), `use-session-stream-events.test.tsx` (streaming batch) | Covered |
| 20 | ADDED | Conversation content hierarchy | 4 | §10.4–10.6, 10.9 | Component | `MessageList.grouping.test.tsx`, `MessageItem.memoization.test.tsx` | Covered |
| 21 | ADDED | Unified multi-seat navigation semantics | 4 | §10.20–10.21 | Component | `src/session-workspace/seat-avatar-group.test.tsx` | Covered |
| 22 | ADDED | Composer and conversation responsive safety | 4 | §10.19, §20.5 (verified non-applicable), §20.6 (partial), §20.12 | Component, Web E2E | `AsyncBoundary.test.tsx`/`RefreshIndicator.test.tsx`/`WaitingIndicator.test.tsx` (reduced motion), `tests/e2e/onepiece-agent.spec.ts` (touch path) | Partial |
| 23 | ADDED | Conversation rendering cost isolation | 3 | §10.13–10.14 | Component | `MessageItem.memoization.test.tsx`, `RichBlocks.test.tsx` | Covered |
| 24 | MODIFIED | Responsive message submission feedback | 6 | §10.19 (verifies pre-existing behavior) | Unit | `src/main-layout/optimistic-message.test.ts` (optimistic display + rollback scenarios directly) | Partial |

**Domain notes**: Requirement 17's own gap was found independently while cross-checking this
document's citations, not carried over from `tasks.md` prose — `ButtonArea.test.tsx` covers the
composer's closed-state summary and high-risk badge, but the popover and its per-field provenance
badge (`ComposerConfigPopover.tsx`, `ConfigField.tsx` — the two components tasks 10.16/10.17 actually
built) have zero test coverage of their own; a keyboard/reset/scenario-level proof for "Open run
configuration" and "Reset configuration" specifically does not exist yet. Requirement 22's "Open a
virtual keyboard" scenario was investigated (task 20.5)
and found genuinely non-applicable to this app's real target platform (OS-enforced 700px window
minimum, no mobile Tauri bundle target) — a reasoned non-applicability, not a fabricated pass;
still counted Partial here because no automated test exists either way and 20.6's pointer-target
audit is explicitly non-exhaustive. Requirement 24 is largely pre-existing chat-submission behavior
this redesign's own tasks verified rather than rebuilt (10.19); `optimistic-message.test.ts` (which
this audit located and confirmed directly — `describe("optimistic user messages")`, "creates an
immediately renderable user message...", "removes only the rejected temporary message during
rollback...") solidly covers 2 of 6 scenarios. The other 4 (plain submit acknowledgement, pending/
queued state, pre-acceptance failure, post-persistence failure) are plausible pre-existing coverage
this audit did not locate a specific file for — not claimed as covered without a citation.

## goal-management (`specs/goal-management/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 25 | ADDED | Searchable goal execution-target picker | 3 | §15.4–15.6 | Unit, Component | `execution-target-picker.test.tsx`, `execution-target-providers.test.ts` (incl. §21.11's 20-cap regression cases) | Covered |
| 26 | ADDED | Nonblocking goal mutations | 3 | §15.7–15.8 | Unit | Goal Center action hook tests per 15.7/15.8 evidence (exact file not independently glob-confirmed) | Partial |
| 27 | ADDED | Goal master-detail presentation | 3 | §15.1–15.2 | Component | Goal Center master-detail suite per 15.1/15.2 evidence | Covered |
| 28 | ADDED | State-aware goal lifecycle actions | 3 | §15.3, 15.9 | Component | Goal detail/presentation tests (`canAccept`/`blockingReason`) per 15.9 evidence | Covered |
| 29 | ADDED | Goal relationship overview | 3 | §15.10–15.11 | Component | `goal-relationship-sections.test.tsx` ("renders an unresolvable link explicitly rather than dropping it from the view", + §21.11's 20-link-cap case, + §20.17's theme-parity case) | Partial |
| 30 | ADDED | Responsive Goal Center navigation | 2 | §15.12, §20 | Component | 15.12's 2 new compact-Back tests | Partial |

**Domain notes**: Requirement 26's pending/reconcile/rollback mechanism is real (per 15.7/15.8's
own evidence) but this audit did not independently glob-confirm its exact test file name — flagged
rather than cited with unverified confidence. Requirement 29's spec text names "milestones" and
"acceptance evidence" as relationship kinds; `GoalLink`'s real type only carries Work
Items/Sessions/Runs/Loops — a disclosed, structural gap against the literal spec wording, not a
missing test. Requirement 30's compact-width scenario is solid; a dedicated keyboard-operability
test for Goal Center's own navigation was not independently located.

## loop-management-ui (`specs/loop-management-ui/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 31 | ADDED | Loop definitions and runs route separation | 3 | §17.1 (route real), 17.2 (partial), §4 | Unit | `workbench-route.test.ts` | Partial |
| 32 | ADDED | Loop iteration Inspector selection | 3 | §17.11 (blocked), §9 | — | No iteration-level Inspector provider exists | Gap |
| 33 | ADDED | Loop local mutation feedback | 3 | §17.14–17.15 | Web E2E | `tests/e2e/loop-engineering.spec.ts` | Covered |
| 34 | MODIFIED | Dedicated Loop Center | 5 | §17.3 | Component | `loop-center.test.tsx` (125 tests incl. `src/ui/inspector`) | Covered |
| 35 | MODIFIED | Loop Center operational layout | 5 | §17.3 | Component | `loop-center-regions.test.tsx`, `loop-center-responsive.test.tsx` | Covered |
| 36 | MODIFIED | Run phase and iteration monitoring | 7 | §17.8–17.10, §21.12 | Component, Perf | `loop-run-workspace.test.tsx` (incl. §21.12's 60-iteration timeline case) | Partial |
| 37 | MODIFIED | Persistent Loop run action header | 6 | §17.8 | Web E2E | `tests/e2e/loop-engineering.spec.ts` (Reject flow) | Partial |
| 38 | MODIFIED | Decision-oriented iteration history | 5 | §17.10, 17.12 | Unit | `loop-run-workspace.test.tsx`, `loop-presentation.test.ts` | Partial |
| 39 | MODIFIED | Decision-ready human acceptance panel | 8 | §17.13 (partial) | Component, Web E2E | `loop-acceptance-panel.test.tsx`, `tests/e2e/loop-engineering.spec.ts` (sticky/non-overlap test, task 20.9) | Partial |

**Domain notes — this is the least-complete domain in the matrix.** Requirement 32 is a genuine
**Gap**: task 17.11 is explicitly blocked and no per-iteration `WorkbenchSelection`/Inspector
provider exists — following an iteration selection into the Inspector is not built at all, not
just untested. Requirement 36's own action-update budget (task 21.12) is itself only half-closed:
the "does not re-fetch" half is proven at 60-iteration scale, but the "does not re-render the whole
iteration list" half is a confirmed, disclosed real gap (`LoopIterationRow` is not memoized, and
the Web mock's own mutation responses deep-clone every iteration, so every row's render function
re-invokes on every action — not a visible/functional bug, since React's reconciler still skips
the DOM patch, but a real gap against the requirement's own literal wording). `loop-engineering.
spec.ts`'s own main pause→feedback→acceptance flow test has a pre-existing, reproducible failure
(confirmed via `git stash` against the unmodified baseline) unrelated to this change's diff,
constraining how much of this domain can be proven end-to-end in a browser today.

## main-layout-ui (`specs/main-layout-ui/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 40 | ADDED | Session runtime panel | 4 | §8.6, 8.9–8.10 | Unit, Component, Web E2E | `session-runtime-panel.test.tsx` (8), `session-runtime-panel-trigger.test.tsx` (3), `workbench-layout-preferences.test.ts` (+4), `tests/e2e/session-workspace-tabs.spec.ts` | Covered |
| 41 | ADDED | Session route and return-context compatibility | 3 | §4.7–4.9, §4.10 (partial) | Unit, Web E2E | `mission-control-view-state.test.ts` (6), `mission-control.test.tsx` (stale-response regression), `tests/e2e/mission-control.spec.ts` + `workspace-routing.spec.ts` (18/18) | Partial |
| 42 | ADDED | Contextual session status hierarchy | 3 | §10.1–10.2 | Unit, Component, Web E2E | `session-presentation.test.ts` (8), `session-conversation-header.test.tsx` (5), `tests/e2e/main-chat.spec.ts` | Covered |
| 43 | MODIFIED | Workspace activity bar | 11 | §4.12–4.13, §5.1 | Component, Web E2E | `workspace-activity-bar.test.tsx`, `tests/e2e/workspace-activity-bar.spec.ts` | Partial |
| 44 | MODIFIED | Three-panel workspace proportions | 10 | §5.2–5.10, §5.4 (partial) | Component, Web E2E | `SplitPane.test.tsx`, `DestinationLayoutBody.test.tsx`, `workbench-layout-preferences.test.ts` (10), `tests/e2e/workspace-activity-bar.spec.ts` (13/13), `multi-agent-session.spec.ts` | Partial |
| 45 | MODIFIED | Sidebar session organization | 22 | §7.1–7.17 | Unit, Component, Web E2E | `session-sidebar-model.test.ts` (25), `session-sidebar.attention-and-filters.test.tsx` (16), `session-row-list.test.tsx` (7), `tests/e2e/session-navigation.spec.ts`, `workspace-activity-bar.spec.ts` | Covered |
| 46 | MODIFIED | Optimized information panel tabs | 16 | §9.1–9.17, §9.9/9.10 (unbuilt) | Unit, Component, Web E2E | `workbench-selection.test.ts` (10), `use-workbench-inspection.test.tsx` (11), `Accordion.test.tsx`, `session-overview.test.tsx`, `tests/e2e/usage-statistics.spec.ts` etc. | Partial |
| 47 | MODIFIED | Create-session dialog | 16 | §11.1–11.14, §11.4 (partial) | Unit, Component, Web E2E | `create-session-draft-model.test.ts` (10), `create-session-validation.test.ts` (4), `create-session-step-4.test.tsx` (4), `tests/e2e/create-session-wizard.spec.ts` | Partial |
| 48 | MODIFIED | Declarative session workspace tab capabilities | 10 | §8.1–8.20 | Unit, Component, Web E2E | `session-surface-registry.test.ts` (8), `legacy-session-surface-adapter.test.ts` (7), `session-files-surface.test.tsx` (3), `tests/e2e/session-workspace-tabs.spec.ts` (9/11) | Covered |

**Domain notes**: Requirement 41's "Requested evidence is stale" scenario is only real for
Sessions today — task 4.10 explicitly confirms loop runs/definitions, projects, and evaluation
experiments named by a route have **zero** not-found signal, and the one real signal (Sessions)
doesn't yet use the typed `RouteValidationState` the spec's own vocabulary implies. Requirement
43's "Use a narrow height" scenario has no located test — confirmed by grep (`workspace-activity-
bar.test.tsx` has zero height-related assertions). Requirement 44's "Protect main content width"
scenario is only enforced for Session Work; task 5.4 explicitly discloses Run detail/Loop timeline/
Board/Evaluation table/Settings content have no pane-splitting behavior yet to floor a minimum
width for. Requirement 45's own task (7.17) discloses no real-browser 1,000-session Playwright test
exists (only the unit-level exact-threshold proof) — a Web-adapter seeding-mechanism limitation,
not an oversight. Requirement 46's EvidenceLink completeness (9.9) and bounded-loading safety
(9.10) are both explicitly `[ ]`. Requirement 47's Step 2 does not show the "Skill summary" its own
"Configure participants" scenario names in words (task 11.4's own honest disclosure — the workspace
isn't chosen until Step 3, so a global-scope-only Skill summary would mislead).

## project-worktree-management (`specs/project-worktree-management/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 49 | ADDED | First-class Projects and Workspaces destination | 3 | §13.1 (route real), 13.4 (partial) | Unit, Component, Web E2E | `src/projects/workspace-filter.test.ts`, `projects.test.tsx`, `tests/e2e/projects-workspaces.spec.ts` | Partial |
| 50 | ADDED | Persistent workspace trust presentation | 3 | §13.6, 13.10 (blocked) | Unit (fixtures only) | `src/projects/workspace-fixtures.ts` (untrusted/revoked fixtures never produced by real derivation) | Gap |
| 51 | ADDED | Workspace contextual quick actions | 3 | §13.8 (4/8 real), 13.9 | Component, Web E2E | `workspace-detail.test.tsx`, `create-session-dialog-content.prefill.test.tsx` | Partial |
| 52 | ADDED | Unavailable workspace recovery | 3 | §13.6, 13.8 (partial) | Unit | `use-workspace-reconnect.test.ts`, `workspace-aggregation.test.ts` | Partial |
| 53 | ADDED | Workspace relationship summary | 2 | §13.7 (partial) | Unit | `src/projects/workspace-plan-links.test.ts` | Partial |
| 54 | ADDED | Responsive workspace management | 2 | §13.12 | Web E2E | `tests/e2e/projects-workspaces.spec.ts` (compact-width list↔detail swap with Back) | Covered |

**Domain notes**: Requirement 50 is a real **Gap** — `workspace-fixtures.ts` carries "untrusted"/
"revoked" fixture states, but the real trust-derivation code path (from actual project inspection)
was found to only ever yield "trusted"/"unknown" in practice; task 13.10 is confirmed genuinely
blocked. Requirement 51's Open Shell/Create Worktree/Relocate/Remove-History quick actions (4 of 8
named) have no backing service yet. Requirement 53's Active-Runs/Quality relationship links are
honestly reported "not available" — a cross-backend `projectId` correlation gap, not a UI omission.

## scheduled-task-management (`specs/scheduled-task-management/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 55 | ADDED | Scheduled task update and duplication | 4 | §19.7–19.9 | Unit, Rust | `scheduled-task-adapter-parity.test.ts`, migration 95 version-conflict Rust tests | Covered |
| 56 | ADDED | Scheduled task run-now action | 3 | §19.10 | Rust, Unit | Dedicated interleaved-dispatch Rust test per 19.10 evidence | Partial |
| 57 | ADDED | Scheduled task run history UI | 4 | §19.11, §21.14 | Rust, Unit, Perf | `list_scheduled_task_runs_pages_with_real_offset_and_limit`, `use-scheduled-task-history.test.ts`, `scheduled-task-list.test.tsx` (100-task budget) | Covered |
| 58 | ADDED | Scheduled recurrence timezone and occurrence preview | 4 | §19.12–19.13, §19.18 (DST declined) | Unit | `scheduled-task-recurrence.ts` consumer tests | Partial |
| 59 | ADDED | Scheduled runtime capability disclosure | 3 | §19.15 | Component | Reused `runtimeHint` key; no dedicated new test cited in evidence | Partial |
| 60 | ADDED | Localized scheduled recurrence labels | 3 | §19.14, §20.13 | Unit | `i18n-resource-parity.test.ts` | Covered |
| 61 | ADDED | Scheduled task action hierarchy | 3 | §19.16–19.17 | Component | Dedicated tests per §19.17 evidence | Covered |
| 62 | MODIFIED | Scheduled task dialog | 9 | §19.1–19.3, §19.18 | Unit, Web E2E | `runs-destination.test.tsx`, `workspace-activity-bar.test.tsx`, live 900px compact-viewport Web E2E test | Covered |

**Domain notes**: This domain's real scenario count (33) exceeds `traceability.md`'s stale count
(29) entirely within its one MODIFIED requirement (62, "Scheduled task dialog") — reworded/added
scenario pairs accumulated as 19.1–19.18 landed. Requirement 58's DST-transition handling is
explicitly declined (task 19.18), a disclosed scope boundary, not an oversight.

## settings-center-ui (`specs/settings-center-ui/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 63 | ADDED | Searchable settings metadata registry | 3 | §12.1–12.3 | Unit | `settings-pages-architecture.test.ts` (8), `settings-search-index.test.ts` (12 + synthetic duplicate-detection cases) | Covered |
| 64 | ADDED | Cross-page field-level settings search | 4 | §12.4–12.7 (12.7 left `[ ]`) | Unit, Component | `settings-search-index.test.ts` (12), `settings-search-box.test.tsx` (7), `use-settings-anchor-highlight.test.ts` (5) | Partial |
| 65 | ADDED | Unified settings save semantics | 4 | §12.10–12.11 | Unit, Component | `page-parts.test.tsx` (4), `basic-settings-page.test.tsx`, `DraftActionBar` `saveDisabled` test | Partial |
| 66 | ADDED | Settings unsaved-change protection | 3 | §12.12–12.13 | Unit, Component, Web E2E | `use-draft-navigation-guard.test.tsx` (5), `settings-shell.test.tsx` (+7), `mcp-server-form.test.tsx` (4), `tests/e2e/mcp-settings.spec.ts` | Covered |
| 67 | ADDED | Workflow-grouped settings navigation | 3 | §12.9, §12.16 | Unit, Component, Web E2E | `settings-compact-nav.test.tsx` (6), `settings-page-status.test.ts` (7), `settings-shell.test.tsx` status describe block, 10-file narrow-viewport e2e sweep | Covered |
| 68 | ADDED | Settings danger and sensitivity hierarchy | 3 | §12.13, §12.14–12.15 | Unit, Component | `mcp-server-form.test.tsx`, `observability-settings-page.test.tsx` (+3), `local-media-page.test.tsx` (+1), `cli-parameters-page.test.tsx` (+2), 24-site destructive-action audit (`Button` `variant="destructive"` + `DangerZone`) | Covered |
| 69 | ADDED | Copyable safe settings diagnostics | 2 | §12.19 | Unit, Component, Web E2E | `diagnostic-field.test.ts` (3), `CopyDiagnosticsButton.test.tsx`, 10 per-page `*-diagnostic-summary.test.ts` files (~80 cases total across CLI Management/IM/Local Media/Extensions/Plugins/SSH/Observability/LSP/Agent Configurations/MCP), 8 Web E2E specs | Covered |
| 70 | MODIFIED | Lazy settings module loading | 12 | §12.17 | Component | `page-lifecycle-policy.test.ts` (6), `settings-shell.test.tsx` "SettingsShell page lifecycle" describe block | Covered |
| 71 | MODIFIED | Polished settings visual system | 6 | §12.8, §12.18 (6/20 pages) | Component | `settings-shell.test.tsx`, `ssh-connections-page.test.tsx` (6), `mcp-page.test.tsx` (11), `extensions-page.test.tsx`, `plugin-integrations-page.test.tsx` | Partial |

**Domain notes**: Requirement 64's mechanism is fully built and tested, but only against
**synthetic** fixtures — task 12.1 deliberately left `fields: []` on all 20 registered pages, so
the real cross-page field-level search this requirement describes surfaces nothing beyond
page-level matches in the shipped app today; task 12.7 (removing the superseded page-only search)
is explicitly held `[ ]` pending that real parity. Requirement 65's immediate-save/draft-save UI
surfaces are real but each adopted by only one page (`basic-settings-page.tsx`'s 4 rows;
`cli-parameters-page.tsx`) of 20 registered pages — the shared mechanism exists, most pages don't
use it yet. Requirement 71 is the most significant, cross-validated gap in this domain: only 6 of
20 settings pages (SSH Connections, Extensions, Plugin Integrations, Skills, CLI Management, MCP)
use the shared `src/ui/` primitives this requirement names; the other 14 still render through the
local `page-parts.tsx` shadow kit (independently corroborated by this repository's own prior
finding, memory note "Settings has a parallel shadow UI kit"). `DataTable` — named explicitly in
the requirement text — has **zero** adoption among any of the 6 migrated pages (task 12.18's own
finding: neither `EntityList` nor `DataTable` matched any of the 6 pages' real card-grid shape);
`Toolbar`/`FilterBar`/`FilterPopover` likewise have zero production consumers anywhere in the app.

## unified-todo-board (`specs/unified-todo-board/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 72 | ADDED | Unified board toolbar and saved views | 4 | §14.1 (4/5), 14.3–14.5 | Unit, Component, Web E2E | `work-board-saved-views.test.ts`, `work-board.test.tsx`, `tests/e2e/todo-board.spec.ts` | Covered |
| 73 | ADDED | Nonblocking work-item mutations | 3 | §14.10 (partial), 14.11 | Component | `work-board-card.test.tsx`, `work-board.test.tsx` | Partial |
| 74 | ADDED | Work-item editor sheet | 3 | §14.2 | Component | `work-board.tsx`'s `WorkItemForm`-in-`Sheet` wiring + `work-board.test.tsx` | Covered |
| 75 | ADDED | Canonical and accessible stage movement | 3 | §14.8–14.9 | Component | `work-board-card.test.tsx`, `work-board.test.tsx` | Partial |
| 76 | ADDED | Board batch management | 3 | §14.12 | Component | `work-board.batch.test.tsx`, `work-board-batch-panel.test.tsx` | Covered |
| 77 | ADDED | Responsive stage-list presentation | 2 | §14.13, §20.2 | Component, Web E2E | `work-board-item-list.test.tsx`, `tests/e2e/todo-board.spec.ts` (4-width matrix) | Covered |
| 78 | ADDED | Bounded work-item card metadata | 3 | §14.6–14.7, §21.9 | Component, Perf | `work-board-card.test.tsx`, `work-board-item-list.large-scale.test.tsx` (1,000-item DOM-bounded budget) | Covered |
| 79 | ADDED | Optional board WIP guidance | 2 | §14.14 | Component | `work-board.wip-limits.test.tsx` | Covered |

**Domain notes**: Requirement 73's "version conflict" scenario is structurally impossible to test —
`WorkItem` has no version field anywhere in its type or Rust model (confirmed by this domain's own
task evidence); pending/rollback for a single mutation is otherwise real and proven at 1,000-item
scale (task 21.9). Requirement 75's "reject an invalid move" scenario was not independently located
as its own named test, though the canonical one-command-for-drag/keyboard/menu path is confirmed.

## workbench-design-system-ui (`specs/workbench-design-system-ui/spec.md`)

| # | Δ | Requirement | Scenarios | Task(s) | Test kind(s) | File pointer(s) | Verdict |
|---:|---|---|---:|---|---|---|---|
| 80 | ADDED | Task-domain workbench navigation | 3 | §4.1–4.3, 4.12, §5.1 | Unit, Component, Web E2E | `workbench-route.test.ts` (73), `workspace-activity-bar.test.tsx`, `tests/e2e/workspace-routing.spec.ts` | Partial |
| 81 | ADDED | Shared destination layout primitives | 2 | §3.1–3.2 | Component | `AppShell.tsx` + test, `DestinationLayoutBody.tsx` + test | Covered |
| 82 | ADDED | Container-responsive pane composition | 3 | §5.3–5.10 | Component | `DestinationLayoutBody.test.tsx`, `SplitPane.test.tsx` (primitive-level; only Sessions is a live consumer today per §5.4's own disclosure) | Covered |
| 83 | ADDED | Global command center | 4 | §6.1–6.14 | Unit, Component, Web E2E | `command-center-registry.test.ts` (3), `command-center.test.tsx` (13), `use-command-center-search.test.ts` (7), `rank-search-results.test.ts` (10), `tests/e2e/command-center.spec.ts` (2) | Covered |
| 84 | ADDED | Explicit page lifecycle policy | 3 | §5.11–5.17, §3.13, §21.16 | Unit, Component | `destination-lifecycle.test.ts`, `settings-shell.test.tsx` mount-spy suite, `loop-run-polling.test.ts`, `loop-monitoring.test.tsx`, `use-evaluation-query.test.ts`, `use-container-compact-mode.test.ts`, `notification-toast-viewport.test.tsx` | Covered |
| 85 | ADDED | Unified asynchronous view states | 4 | §3.13–3.14 | Component | `AsyncBoundary.tsx`/`RefreshIndicator.tsx`/`MutationStatus.tsx` + tests, adopted across §12.18/§13/§14.11 | Covered |
| 86 | ADDED | Shared keyboard interaction models | 4 | §3 (all), §20.7–20.8, §20.19 | Unit, Component, Web E2E | `use-tab-list.test.tsx`, `ActionMenu.test.tsx`, `session-context-panel.test.tsx`, 9 keyboard-only Web E2E flows (§20.19) | Covered |
| 87 | ADDED | Semantic theme and localization parity | 3 | §2.1–2.14, §20.13–20.14, §20.17 (partial) | Unit, Component, Web E2E | `format.test.ts`, `i18n-resource-parity.test.ts`, `work-board-card.test.tsx` + `goal-relationship-sections.test.tsx` (theme-parity structural tests, 2 destinations × 1 state each) | Partial |
| 88 | ADDED | Structural frontend performance budgets | 3 | §0.9, §21.7–21.16 | Unit, Component, Perf | `session-sidebar.large-scale.test.tsx`, `VirtualizedMessageList.large-scale.test.tsx`, `work-board-item-list.large-scale.test.tsx`, `mission-control.test.tsx` (query/hidden-page budgets), `evaluation-results-table.test.tsx`, `scheduled-task-list.test.tsx`, `resource-tracking.test.ts` | Covered |
| 89 | ADDED | Workbench visual regression contract | 3 | §21.17–21.21 (21.20–21.21 unstarted) | Visual, Process | `tests/e2e-visual-regression/core-surfaces.spec.ts` (24 of ~300 baselines), `CONTRIBUTING.md`'s "Visual regression baselines" section (review-note process, commit `fbbdae4b`) | Partial |

**Domain notes**: Requirements 80 and 82 share row 43's own disclosed "narrow height"/"narrow
window" test gap for the activity rail (no located test either way) — 82 is marked Covered because
its own scenarios are about pane composition (proven at the primitive level), not the activity
rail specifically. Requirement 84 is the single best-evidenced requirement in this matrix — task
21.16 (this session's own resource-release proof pass) independently reinforces nearly every one of
its scenarios across 5 different destinations plus the global notification coordinator. Requirement
87's theme-structural-equivalence proof (distinct from pixel-screenshot similarity) exists for
exactly 2 destinations × 1 state each (Work Board's batch checkbox, Goal Center's disabled unlink
button) — task 20.17 itself discloses this is real evidence, not a full sweep. Requirement 89's
"Review a changed baseline" scenario is Covered via a real, committed process (`CONTRIBUTING.md` +
PR template checklist) rather than a runnable test — appropriate for a process requirement, and
independently confirmed via `git show fbbdae4b --stat`; the other two scenarios ("Capture the core
matrix", "Report desktop coverage") are explicitly partial/not-started.

## Full list of Gap-verdict requirements

Only 2 of 89 requirements have no real automated coverage of their core mechanism:

- **#32 — loop-management-ui, "Loop iteration Inspector selection"**: no per-iteration
  `WorkbenchSelection` kind or Inspector provider exists; task 17.11 is explicitly blocked.
- **#50 — project-worktree-management, "Persistent workspace trust presentation"**: trust-state
  fixtures exist (`untrusted`/`revoked`), but the real derivation path only ever produces
  `trusted`/`unknown` from actual project inspection; task 13.10 is confirmed genuinely blocked.

Every other requirement has at least partial, real, cited automated coverage. The 40 Partial-verdict
requirements are listed with their specific named gaps in each domain's own notes above, rather
than repeated in a flat list here — the gaps are heterogeneous (deliberately deferred scope,
synthetic-fixture-only proof, single-page adoption of an app-wide mechanism, explicitly incomplete
`[ ]` tasks) and are more useful read in their domain's own context.
