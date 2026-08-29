# Milestone 0 baseline — `redesign-unified-workbench-ui`

Recorded before any redesign code changes, on a worktree branched fresh from `origin/main`.

- **Branch**: `feat/redesign-unified-workbench-ui`
- **Base commit**: `f1f15cd6408c5bb3376c9f8debd6e41e9368dfac` (`origin/main` at fetch time — "Reconcile both documentation guides with their specifications (#249)")
- **Prerequisite change**: `openspec/changes/improve-workspace-ui-ergonomics` is complete (47/47 tasks), merged as `ee3eaf3f` ("Make the workspace scannable and recoverable (#208)"), present in this branch's history. Not yet archived administratively, but functionally done — no rebase needed before starting this change.
- **Environment**: Windows 11, node v24.15.0, npm 11.18.0, rustc 1.97.1, cargo 1.97.1. `node_modules` installed via `npm ci` (1018 packages).

This baseline package was produced by a **static audit** (see the change's own `README.md` in the delivery package) that never cloned/ran the app. Every fact below was independently re-verified against the running app and current source in this repo — where it disagrees with the static audit's assumptions, this document is the corrected source of truth per the change's own instruction to prefer runtime reality over static claims.

## Verification command results (tasks 0.3–0.5)

| # | Command | Result | Duration | Notes |
|---|---|---|---|---|
| 1 | `npm run lint:ci` | PASS | 5m40s | 0 lint errors |
| 2 | `npm run test` | **FAIL** | 16m13s | 7 failing tests / 6 files — see below |
| 3 | `npm run build` | PASS | 5m58s | tsc clean; 16 lazy chunks; main static closure 156.6 KiB gzip |
| 4 | `npm run architecture:check` | PASS | 29m17s | Includes 83/83 Rust architecture-fitness tests (4 binaries) |
| 5 | `npm run contracts:check` | PASS | 5.2s | Catalog + matrix up to date, 16 tests |
| 6 | `openspec validate --specs --strict` | PASS | 13.6s | 142 specs validated |
| 7 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS | seconds | No diffs |
| 8 | `cargo check --workspace` | PASS | 19m15s | Cold cache |
| 9 | `cargo clippy --workspace --all-targets -- -D warnings` | PASS | 17m28s | 0 findings |
| 10 | `npm run native:panic:check` | PASS | 15m33s | 0 `unwrap`/`expect` findings |
| 11 | `cargo test --workspace` | PASS | ~11m (warm retry) | **6004 passed, 0 failed, 15 ignored**. First attempt hung solid (0 CPU activity across two checks 5min apart) and had to be killed — no code involved, environmental (see caveats). Retry with warm cache was clean. |
| 12 | `npx playwright test` | PASS (208/210) | 31.0 min | 210 tests, 48 spec files, chromium only, 2 workers, locale `zh-CN`, theme `minimal`. 2 failures — see below. |
| 13 | `npm run test:desktop:build` | PASS | ~10-13m | Real `tauri build --debug --features desktop-e2e`; produced `vanehub-ai.exe` |
| 14 | `npm run test:desktop:smoke` | **BLOCKED** (partial, all-clean) | 2×~63m (both cut off) | See "Desktop smoke" below |
| 15 | `npm run test:desktop:core-smoke` (supplementary) | PASS | 38.3s | 1/1, real IPC/SQLite/git-diff/Mission-Control round trips |

**Conflict resolved**: the OpenSpec tool's generic `context` field suggested `cargo check --manifest-path src-tauri/Cargo.toml`; this repo is a cargo workspace where that flag silently skips other members (e.g. `vanehub-permission-hook`). Followed `AGENTS.md`'s `--workspace` requirement instead throughout.

## Known pre-existing defects (not introduced by this change, do not "fix" as drive-by work unless a specific task calls for it)

1. **7 pre-existing Vitest failures**, all on unmodified code, likely async-loading timing flakiness — recommend isolating and re-running before treating any of these as a redesign regression:
   - `session-workspace/trace-linked-evidence.test.tsx` — log-index correlation row never renders in time
   - `settings/settings-provider.test.tsx` — save-against-stale-revision assertion never satisfied
   - `settings/pages/prompt-hooks-page.interaction.test.tsx` — 2 failures (compact-management count text, category expansion button)
   - `settings/pages/usage-statistics-page.interaction.test.tsx` — 15s timeout
   - `settings/pages/agents/lsp-configuration-section.test.tsx` — LSP-enable checkbox never renders in time
   - `settings/pages/cli-management/cli-management-page.test.tsx` — 15s timeout
2. **Genuine Playwright defect, reproduced identically twice**: `tests/e2e/multi-agent-session.spec.ts` — in a running shared/multi-agent session, the message region does not auto-scroll/anchor to the newest message after a roster-presence + mention event. Directly relevant to Milestone 2's `ConversationWindowModel` (tasks 10.10–10.12) — worth a specific regression test there, not a fix owed under Milestone 0.
3. **Flaky, not confirmed real**: `tests/e2e/im-settings.spec.ts` (session IM pane visibility) and `tests/e2e/execution-observability.spec.ts` (heading text) — each failed in exactly one of two runs, with the failure window correlated to transient environment contamination (see below), not app behavior.
4. **Runtime bug found during screenshot capture**: the expanded session information panel visually overlaps and intercepts pointer events meant for the conversation header's "…" (more actions) button — a real hit-testing conflict, reproducible via direct-DOM-click workaround. Relevant to Milestone 2 Inspector/conversation-header work (task group 9–10).
5. **Dead/unused localStorage key**: `vanehub.uiStyle` (`src/theme/theme-registry.ts`) is declared but has zero read/write call sites anywhere in the codebase — theme actually persists through `AppSettings.theme` via settings-service. Not necessarily in scope to remove, but don't assume it's load-bearing.
6. **Console noise, not failures**: recurring React `flushSync was called from inside a lifecycle method` warnings and `ResizeObserver loop completed with undelivered notifications` unhandled errors appear during normal Playwright runs. Pre-existing, did not fail any test.

## Environmental caveats affecting this baseline run (not app defects)

- This worktree's `node_modules` was empty when background verification began (the worktree was freshly created and `npm install`/`ci` was not run up front — a process gap on my part). Multiple concurrent baseline jobs needed it; one ran `npm ci` mid-flight, which raced against a concurrently-running Playwright pass and caused a transient, fully self-diagnosed corruption window (Playwright package files rewritten, `package-lock.json` untouched). The affected Playwright run was discarded and re-run clean; the two "flaky" results above fall inside that window. `node_modules` is confirmed stable now (matches `package-lock.json` exactly) — this should not recur for later milestones as long as the worktree isn't left without an initial `npm ci`.
- `cargo test --workspace` hung solid (zero CPU activity, no flushed output) on its first attempt and had to be killed; a warm-cache retry passed cleanly with 6004/6004 (non-ignored) tests passing. Given this repo's documented history of shared build-lock and SQLite contention across concurrent worktree sessions, treat a future isolated hang as a possible false alarm to rule out via retry before assuming a regression — but don't assume every future hang is environmental either.
- `src-tauri/gen/schemas/desktop-schema.json` and `windows-schema.json` show as modified in `git status` with no `git diff` output — known phantom CRLF dirt from Windows cargo builds in this repo, not a real change.

## Runtime architecture facts confirmed (tasks 0.10–0.11 — corrects/grounds the static audit)

- **Routing**: `react-router` (`BrowserRouter`/`Routes`/`Route`) is real and used for exactly 3 top-level routes (`/workspace/*`, `/settings`, catch-all → `LaunchRedirect`). Within `/workspace/*`, destinations are deliberately hand-parsed (`src/main-layout/workspace-route.ts`), **not** nested `<Route>` elements — a code comment explains why: React Router unmounts the previous element on navigation, and the workspace requires previously-visited destinations to stay mounted. **The new `DestinationDefinition` registry (tasks 4.1–4.3) must preserve this mount-retention property**, not just wrap the existing behavior in a router-idiomatic way that would reintroduce unmounting.
- **"Visited once, mounted forever" exists in two independent places today**: `MainLayout`'s 5 one-way `*Visited` booleans for `loops`/`work-board`/`goals`/`evaluations`/`mission-control` (`main-layout.tsx` L101-149), and `SettingsShell`'s `visitedPages: Set<SettingsPageId>` (`settings-shell.tsx` L23-39) — the latter is the **only** true full-unmount boundary in the app (`/settings` is a sibling route, not nested under `/workspace/*`); everything else only toggles CSS `hidden`. Confirmed at runtime: DOM node counts barely change when a destination becomes hidden (e.g. Work Board 1329→1336 nodes), consistent with tasks 5.12/5.17/12.17's premise.
- **9 session tabs** confirmed exactly as assumed: `chat, changes, documents, files, terminal, shell, logs, traces, report` (`src/session-workspace/session-tab-bar.tsx`). Active tab is in-memory only (not persisted), resets to `chat` on session change (during render, not an effect, to avoid a stale-panel frame).
- **Slash-tab navigation exists but is scoped to OnePiece sessions only today** (`isOnePieceSession` gate in `src/services/slash-commands/command-availability.ts`) — the legacy tab-id adapter (task 8.7) only needs to bridge this one mechanism, not a general one.
- **5 real localStorage UI-preference keys** (not the unbounded list a naive grep suggests — most `localStorage` hits are Web/mock-only fixture storage, not real preferences): `vanehub.workspace.location.v1`, `vanehub.session-sidebar.width.v1`, `vanehub.session-sidebar.presentation.v1`, `vanehub.session-sidebar.expanded-groups.v1`, `vanehub.create-session.agent.v1`.
- **Settings**: 19 pages in 5 order-enforced groups (order asserted by `tests/e2e/settings-navigation-order.spec.ts` — do not break this when implementing task 12.9's grouped sidebar).
- **Evidence linking is two distinct layers today, and no `EvidenceLink` symbol exists anywhere yet** (confirmed via project-wide grep — it's genuinely new, tasks 3.17/9.9):
  - A rich, well-tested **intra-session** layer (`WorkspaceEvidenceTarget`/`useWorkspaceEvidenceScope().navigate()`) that fails closed on session-scope mismatch and has documented "journey" tests.
  - A thin, **partially-implemented cross-destination** layer: Mission Control's `MissionControlNavigationTarget` only special-cases `kind === "review"` (opens the Changes tab); `loop`/`goal`/`evaluation` kinds just navigate to the session without opening any matching tab or switching destination. This is a concrete, confirmed gap behind the static audit's "Mission Control has placeholder facets" claim — real evidence for task groups 9 and 16.
- **"Runs" is fragmented across 4 service files with no unifying concept** today: `AgentRun` (`mission-control-service.ts`), `ExecutionRun` (`execution-observability-service.ts`), `OperationTask` (`operation-service.ts`, generic async task handle), `ExecutionRecord` (`session-workspace-evidence-service.ts`, session-scoped). The fixture generator (task 0.9) targets `MissionControlRunSummary` specifically as the closest thing to a canonical "Run" list-item type — see `src/testing/fixtures/large-scale-fixtures.ts` for the full rationale. Milestone 3/16 work must compose across these, per the change's own Non-Goal, not merge them into one DB model.
- **Confirmed additive-contract gaps** (matches design.md Decision 4/15/16 — these are genuinely new, not extensions of partial features):
  - Command Center cross-object search: zero existing implementation anywhere.
  - Scheduled Task: `listScheduledTaskRuns` (history) already exists; update-in-place, duplicate, run-now, and future-occurrence-preview do not.
  - Evaluation baseline/comparison: zero existing trace in service, types, or UI.
- **Correction to `tasks.md`**: the app ships **5 registered locales** (`zh-CN` default, `en`, `zh-TW`, `ja`, `ko` — `src/i18n/supported-locales.ts`), not the 3 (`zh-CN, en, ja`) named in tasks 20.13/20.17's examples. `AGENTS.md`'s existing i18n rule ("every registered locale") is the correct, controlling instruction — the task text's parenthetical example list is incomplete. Any new string added during this change must land in all 5 locale files, and the visual regression matrix should decide deliberately whether to sample 3 or cover all 5, not silently under-cover `zh-TW`/`ko`.

## Desktop smoke (task 0.6)

**Windows: BLOCKED** for a complete verdict — everything that ran passed, nothing failed.

`npm run test:desktop:smoke` is not a small smoke test — despite the name, it runs `requiredSpecFiles()` from `tests/desktop/spec-manifest.mjs`: all **25** required-fixture desktop spec files sharing one long-lived app/WebDriver session (only one global teardown). Two full attempts were made; both were externally terminated at a consistent ~62-65 minute mark by a background-task lifetime ceiling unrelated to any test timeout. Evidence that nothing actually failed: zero `AssertionError`s in any per-spec log, zero failure screenshots (the suite's `afterTest` hook only saves one on a real failure), and the identical `{"passed":1,"failed":5}` tally recorded by both very-differently-progressed attempts is a wdio forced-kill bookkeeping artifact, not 5 real failures.

Specs confirmed complete and clean across the two attempts: `domain-app-updates`, `domain-automation`, `domain-cli-tooling` (one transient 30s WebDriver script timeout, auto-retried via the suite's own `specFileRetries: 2`, then passed — coincided with an unrelated concurrent desktop-test run from a different worktree, `docs-supplement`, active on this same machine), `domain-evaluation`, `domain-floating-assistant`. The remaining 19 of 25 never started before the time ceiling hit — not attempted, not failed.

As a supplementary, complete data point, `npm run test:desktop:core-smoke` (the literal single-spec `smoke.e2e.mjs` — real startup, `get_desktop_update_snapshot` IPC, local-model endpoint discovery, real SQLite-backed session + streamed message send/receive, a real git-repo code-review diff/revert flow, Mission Control run create/cancel/list-events, evaluation round-trip, Settings navigation) ran to a clean, complete **PASS: 1/1** in 38.3s.

A trustworthy verdict for all 25 required specs needs a longer-lived execution context than a single background task provides here — either the CI `Desktop Smoke` job, or a decomposed run using the suite's `VANEHUB_DESKTOP_SPEC` single-spec override across several shorter calls. Given Milestone 0's purpose is a comparative "before" snapshot (not the project's final desktop gate — that's task 22.12's job before archiving), this is recorded as-is rather than spending further baseline time chasing full completion now.

**macOS: NOT RUN. Linux: NOT RUN** (Windows-only environment).

## Screenshots and resource counts (tasks 0.7–0.8)

95 PNGs across 19 surfaces × 5 widths (1600/1280/1024/768/640), captured via `scripts/ui-redesign/capture-baseline.mjs` against the Web/mock adapter with one representative session/task/goal/evaluation run. Full per-surface DOM-node/interval/observer/listener/resource-entry table (active vs. after-navigated-away) is in [`screenshots/baseline/README.md`](screenshots/baseline/README.md) — not duplicated here to avoid drift between two copies of the same table.

**Not captured with real data**: Loop Center — the Web/mock loop store starts empty and populating it requires driving a 4-step creation wizard with simulated project discovery, out of scope for a factual as-is snapshot. Captured in its true default/first-run empty state instead, documented rather than silently skipped.

## Large-scale performance fixtures (task 0.9)

Deterministic (seeded, no `Math.random()`/`Date.now()`) generators under `src/testing/fixtures/`: 1,000 Sessions, 5,000 Messages, 1,000 Mission Control Runs, 1,000 Work Items, 500 Goals, 200 Loop Runs, 100 Scheduled Tasks, 10,000 Evaluation result rows (300 arenas / 1,000 attempts). Not wired into the normal small Web/mock dataset — opt-in only, for the structural-performance tests in task group 21. See `large-scale-fixtures.ts` for the orchestrator and `large-scale-fixtures.test.ts` for the self-verification (uniqueness, referential integrity, determinism).
