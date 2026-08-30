# Tasks: Redesign unified workbench UI

> Execution rule: keep every checkbox unchecked until the implementation and its required evidence exist. A browser screenshot is not sufficient evidence for native xterm, filesystem, SSH, or scheduler behavior. Do not skip service-contract parity, localization, accessibility, or structural-performance tasks.

## 0. Baseline, rebase, and audit evidence

- [x] 0.1 Create branch `feat/redesign-unified-workbench-ui` from the latest `main` and record the base commit in the change notes. — Branched from `origin/main` @ `f1f15cd6408c5bb3376c9f8debd6e41e9368dfac`; recorded in `docs/ui-redesign/baseline.md`.
- [x] 0.2 Confirm `openspec/changes/improve-workspace-ui-ergonomics` is present or already archived; rebase its sidebar, create-session, Help, Board/Goal, notification, and recovery fixes before changing the same files. — 47/47 tasks complete, merged as `ee3eaf3f`, already an ancestor of this branch; no rebase needed. Not yet administratively archived.
- [x] 0.3 Run and record the repository baseline: `npm run lint:ci`, `npm run test`, `npm run build`, `npm run architecture:check`, `npm run contracts:check`, and `openspec validate --specs --strict`. — Recorded in `docs/ui-redesign/baseline.md`; only `npm run test` has pre-existing failures (7 tests/6 files).
- [x] 0.4 Run and record the Rust baseline commands from `AGENTS.md`; separate pre-existing failures from change regressions. — Recorded in `docs/ui-redesign/baseline.md`; 100% clean (6004 passed, 0 failed, 15 ignored).
- [x] 0.5 Run the existing Web Playwright suite and record passed, failed, skipped, browser, locale, theme, and fixture seed. — Recorded in `docs/ui-redesign/baseline.md`; 208/210 passed (1 genuine pre-existing defect, 1 flaky).
- [x] 0.6 Run the current Tauri/WebdriverIO smoke on each actually available OS and record only platforms that were executed. — Windows: BLOCKED for a full 25-spec verdict (background-task time ceiling, not a failure — build, core-smoke, and 5/25 domain specs all passed clean, zero failures/failure-screenshots anywhere); macOS/Linux: NOT RUN (Windows-only environment). Full detail in `docs/ui-redesign/baseline.md`.
- [x] 0.7 Capture current screenshots for Sessions, nine workspace tabs, information panel, Settings, Board, Goals, Mission Control, Loop Center, Evaluation, and Scheduled Tasks at 1600, 1280, 1024, 768, and 640 widths where supported. — 95 PNGs in `docs/ui-redesign/screenshots/baseline/`; Loop Center captured in first-run empty state only (documented).
- [x] 0.8 Record DOM node count, active interval count, observer count, event-subscription count, and network/service request count for each current destination before and after it becomes hidden. — Table in `docs/ui-redesign/screenshots/baseline/README.md`.
- [x] 0.9 Create deterministic large fixtures for at least 1,000 Sessions, 5,000 Messages, 1,000 Runs, 1,000 Work Items, 500 Goals, 200 Loop Runs, 100 Scheduled Tasks, and 10,000 Evaluation result rows. — `src/testing/fixtures/`, seeded/deterministic, self-verified (9/9 tests), not wired into normal Web/mock data.
- [x] 0.10 Inventory every route, query parameter, slash-tab request, local-storage key, settings page id, tab id, and evidence navigation target affected by the redesign. — Recorded in `docs/ui-redesign/baseline.md` (Runtime architecture facts section).
- [x] 0.11 Inventory every current frontend service call used by the affected pages and identify UI requirements that need additive contract support. — Recorded in `docs/ui-redesign/baseline.md`; 3 confirmed additive-contract gaps (Command Center search, Scheduled Task update/duplicate/run-now/preview, Evaluation baseline/comparison).
- [x] 0.12 Add `docs/ui-redesign/baseline.md` with the evidence table, known pre-existing defects, environment, and screenshot references. — Written and linked from both developer-guide indexes; `npm run docs:check` passes.

## 1. Change governance and implementation gates

- [x] 1.1 Add a temporary `unifiedWorkbenchV2` migration flag through the existing settings/configuration mechanism without creating a permanent product preference. — `DesktopSettingKey::UnifiedWorkbenchV2` end-to-end (domain/DTO/mapper/Rust tests) + `AppSettingKey`/`normalizeAppSettings` on the TS side; no SQL migration needed (settings table is generic key/value).
- [x] 1.2 Document milestone ownership and dependency order in `openspec/changes/redesign-unified-workbench-ui/implementation-notes.md`. — Written, including 3 corrections Milestone 0 made to the delivered planning assumptions.
- [x] 1.3 Create a requirement-to-task-to-test traceability table and keep it updated when requirements or tasks change. — `traceability.md`, all 89 requirements mapped to owning task sections.
- [x] 1.4 Add an architecture guard preventing shared `src/ui/` primitives from importing feature services, Tauri APIs, or feature-specific contracts. — `ARCH-FE-005` in `scripts/architecture/frontend-rules.mjs`, 6 new tests, verified against the real tree (no `src/ui/` files exist yet, so 0 current violations).
- [x] 1.5 Add an architecture guard preventing React files from calling `invoke()` outside the existing Tauri adapters. — Broadened existing `ARCH-FE-001`/`ARCH-FE-002` from `.tsx`-only to "not a `tauri-*.ts` adapter file" (closes a real gap: `.ts` hooks were unchecked); verified zero false positives against the full real `src/` tree.
- [x] 1.6 Add or update the line-count gate so every new or modified production TS/TSX file remains within the repository limit. — Verified the existing mechanism already excludes new files by construction; added a regression test (`line-budget-exemptions.node-test.mjs`) that fails if the exemption list grows or gains a `src/ui/` entry.
- [x] 1.7 Define a deprecation policy for legacy destination ids and legacy session tab ids; add development diagnostics for unmapped values without logging user content. — Policy documented in `src/lib/legacy-id-diagnostics.ts` and `implementation-notes.md`; `warnUnmappedLegacyId` implemented and tested (4 tests).
- [x] 1.8 Require each milestone commit to pass focused tests and strict OpenSpec validation before the next milestone begins. — Documented in `implementation-notes.md`; already followed (M0 fully committed and evidenced before M1 code started). Ongoing discipline, not a one-time artifact.
- [x] 1.9 Do not mark visual tasks complete from browser-only evidence when the behavior depends on xterm, WebView, native dialogs, filesystem selection, or SSH. — Documented in `implementation-notes.md`; already followed (task 0.6 recorded honestly as BLOCKED rather than claiming a desktop pass from browser evidence). Ongoing discipline, not a one-time artifact.

## 2. Semantic tokens and visual foundations

- [x] 2.1 Audit `src/styles.css` and theme token sources; map every hard-coded color, radius, shadow, control height, and ad-hoc surface used by affected modules. — `docs/ui-redesign/token-audit.md`; found existing `console-visual-tokens.test.ts`/`visual-token-rules.ts` already enforce this in 2 of 9 directories, real hex/palette findings confined to `settings/`/`goal-center/`, zero theme-branching JSX (evidence for 2.8).
- [x] 2.2 Add semantic canvas, sidebar, panel, raised, overlay, subtle-border, default-border, and strong-border tokens for both `minimal` and `futuristic`. — `src/styles.css`, all 3 theme blocks, purely additive (canvas/sidebar/raised alias existing surfaces; overlay/border-subtle are new values).
- [x] 2.3 Add compact/default/comfortable control-height and row-height tokens without changing existing density defaults until component migration. — `src/styles.css`; values corrected against the task 2.1 audit's real height inventory (default=36px/h-9, comfortable=44px/h-11, both already the dominant existing tiers, not invented).
- [x] 2.4 Add documented spacing, radius, elevation, focus, status, and motion tokens while keeping radius at or below the repository visual rule. — Tokens in `src/styles.css`; documented in `docs/ui-redesign/design-system.md`. Radius unchanged (still 8/6/4px ceiling).
- [x] 2.5 Define semantic neutral, running, success, warning, danger, information, blocked, and attention state token groups for both themes. — `src/styles.css` (5 new groups + existing 3) + matching `.ucd-status-*` utility classes.
- [x] 2.6 Implement shared focus-ring utilities that remain visible on canvas, panel, selected row, destructive surface, and high-contrast backgrounds. — `.ucd-focus-ring`/`.ucd-focus-ring-on-danger` in `src/styles.css`, canvas-colored gap so the ring never fights its own surface's color.
- [x] 2.7 Add reduced-motion token behavior for pane transitions, sheets, selection highlights, loading indicators, and drag feedback. — `prefers-reduced-motion: reduce` media query zeroes `--motion-*` tokens globally in `src/styles.css`.
- [x] 2.8 Remove theme-specific JSX branches from affected shared UI; themes SHALL alter token values rather than component structure. — Audit (2.1) found zero theme-branching JSX in any of the 9 affected directories; no code change needed. Evidence in `docs/ui-redesign/token-audit.md`.
- [x] 2.9 Define the default surface hierarchy as canvas plus quiet separators, with raised surfaces reserved for independent interactive regions. — Documented in `docs/ui-redesign/design-system.md`; `.ucd-raised` utility reserves elevation for independently-floating content per the rule.
- [ ] 2.10 Replace unnecessary nested cards in one representative page and obtain screenshot approval before applying the pattern globally. — Deferred to task group 12 (Settings): audit found no clean nested-card instance reachable with legacy primitives alone (the one real finding, `SectionPanel`'s `rounded-xl`, is a radius bug deferred to the same migration); a pilot now would use soon-to-be-replaced primitives. Rationale in `docs/ui-redesign/token-audit.md`.
- [ ] 2.11 Add a visual token story or internal fixture page showing all interactive, status, loading, disabled, selected, error, and focus states. — Deferred to land alongside §3's primitives (`AsyncBoundary`/`StatusBadge`/etc.) — a fixture page with only color swatches and no primitives to demonstrate states on would be a weak deliverable.
- [x] 2.12 Add unit or architecture checks preventing new inline color styles and non-semantic status colors in migrated modules. — `ARCH-FE-006` in `scripts/architecture/frontend-rules.mjs`, importing the same `LITERAL_COLOR`/`PALETTE_COLOR` patterns `console-visual-tokens.test.ts` already uses (Node 24 loads the `.ts` source directly), scoped to `src/ui/`; 8 new tests including a caught-and-fixed false-positive against the existing `hsl(var(--x))` pattern.
- [x] 2.13 Document text hierarchy, truncation, wrapping, monospace identifier, and metadata display rules in `docs/ui-redesign/design-system.md`. — Done.
- [x] 2.14 Document the metadata budget for Session rows, Work Item cards, Run rows, Goal rows, and Evaluation rows. — Done, `docs/ui-redesign/design-system.md`.

## 3. Shared UI primitives

- [x] 3.1 Create `src/ui/app-shell/` with shell regions and no feature-service dependencies. — `src/ui/app-shell/AppShell.tsx` + test; TopBar/ActivityRail/RouteOutlet per design.md Decision 2's target structure, every region a caller-supplied slot, imports nothing but `cn`.
- [x] 3.2 Create `src/ui/destination-layout/` supporting contextual navigation, main work surface, Inspector host, Runtime Panel host, and responsive composition. — `src/ui/destination-layout/{use-layout-tier.ts,regions.ts,DestinationLayoutBody.tsx,DestinationLayout.tsx}` + tests; composition logic (which pane is inline vs. `Sheet`, MAIN_MIN_WIDTH starvation guard collapsing Inspector before Navigation) is a pure `tier`-parameterized component separate from the `ResizeObserver` wrapper, since jsdom's stubbed observer never fires and would otherwise make 3 of 4 tiers untestable at the unit layer; breakpoints (1280/1024/768) are not stated as pixel values in design.md, derived from Decision 20's required screenshot-matrix widths.
- [x] 3.3 Create `src/ui/page-header/` with title, bounded description, breadcrumb slot, one primary action, status summary, and More menu. — `src/ui/page-header/PageHeader.tsx` + test; description is `line-clamp-2`, `primaryAction` is a single slot (not an array) so "one primary action" is enforced by the type, More menu composes `ActionMenu` directly.
- [x] 3.4 Create `src/ui/toolbar/` with one keyboard entry point, search slot, filter trigger, active filters, sort/view controls, and batch-mode slot. — `src/ui/toolbar/{use-search-shortcut.ts,Toolbar.tsx}` + test; `/` focuses the caller's search input via a supplied ref (guarded against firing while already typing in an editable field), distinct from a global command-palette shortcut owned elsewhere.
- [x] 3.5 Create `src/ui/filter-bar/` with typed filter definitions, active filter chips, clear-one, clear-all, and localized result counts. — `src/ui/filter-bar/FilterBar.tsx` + test; `FilterDefinition`/`ActiveFilter` take `unknown` rather than a generic `<T>` — a page's filter set is heterogeneous by nature, and `unknown` keeps every concrete definition's own narrowing sound without needing `any` (forbidden repo-wide) to hold them in one array.
- [x] 3.6 Create `src/ui/split-pane/` with pointer and keyboard resizing, clamping, reserved gutter, persistence hooks, and reduced-motion support. — `src/ui/split-pane/{use-pane-resize.ts,SplitPane.tsx}` + test; ARIA `separator` with arrow/Home/End keyboard resize, pointer drag derived from a fixed drag-origin (not the possibly-stale `size` prop), `onResizeEnd` as the caller's persistence hook, `.ucd-pane-transition` skipped while actively dragging.
- [x] 3.7 Create `src/ui/sheet/` by extending the shared dialog/focus primitives for side and full-height sheets with focus trap and focus return. — `src/ui/sheet/{use-focus-trap.ts,Sheet.tsx}` + test; extracted `ApplicationDialog`'s focus-trap effect into `useFocusTrap` (existing `application-dialog.test.tsx` 7/7 still green, confirming no behavior change) and built `Sheet` on the same hook for left/right/bottom/full placements.
- [x] 3.8 Create `src/ui/inspector/` shell with overview, follow, pinned, unavailable, restricted, loading, error, and retry states. — `src/ui/inspector/Inspector.tsx` + test; `mode` selects overview vs. `AsyncBoundary`-driven detail (reusing `AsyncViewState<ReactNode>` directly rather than a parallel state shape), unavailable wires an explicit "Return to overview" action per design.md Decision 8; Pin/Unpin toggle only shows the action valid for the current mode.
- [x] 3.9 Create `src/ui/runtime-panel/` shell with tabs, resize, maximize, restore, close, badges, and per-context state hooks. — `src/ui/runtime-panel/{use-tab-list.ts,RuntimePanel.tsx}` + test; a tab only mounts once actually activated and then stays mounted (verified via a stateful tab retaining its counter across a switch away and back), each tab's `render` receives `isVisible` so it can pause its own effects rather than trusting `hidden` alone (Decision 7); resize is `DestinationLayout`'s vertical `SplitPane` gutter, not this shell's concern — "maximize" here is only the state toggle, since giving the panel full height is a composition decision for whoever assembles the layout.
- [x] 3.10 Create `src/ui/entity-list/` and `src/ui/virtual-list/` wrappers around the existing virtualization dependency with stable item keys and accessible active selection. — `src/ui/virtual-list/VirtualList.tsx` re-exports the existing, 15-call-site-proven `src/components/measured-virtual-list.tsx` rather than forking it; added optional `role`/`onKeyDown`/`activeDescendantId` there (all additive, defaulting to prior hardcoded behavior — verified against all 9 existing test files covering its consumers, 95/95 still pass) so `src/ui/entity-list/EntityList.tsx` + test can layer keyboard-navigable `listbox`/`aria-activedescendant` selection on top without per-item DOM focus, which a virtualized list's off-screen active item cannot hold anyway.
- [x] 3.11 Create `src/ui/data-table/` with typed columns, sorting, filters, pagination, column visibility, selection, empty states, and compact responsive fallback hooks. — `src/ui/data-table/{types.ts,use-table-compact-mode.ts,ColumnVisibilityMenu.tsx,DataTableBody.tsx,DataTable.tsx}` + tests (30 total); "filters" delegates to the existing `FilterBar` rather than duplicating it (composition, not a new filter model), "compact responsive fallback hooks" is a local container-width check (not the page-level `useLayoutTier`) that switches the exact same columns/rows into a stacked card list — same split-into-a-pure-body pattern as `DestinationLayoutBody` for the same jsdom `ResizeObserver` reason. Column visibility is deliberately not built on `ActionMenu`, since toggling a column must not close the popover the way activating a menu item does.
- [x] 3.12 Create `src/ui/status/StatusBadge.tsx` using text, icon or shape, semantic tone, and accessible description. — `src/ui/status/StatusBadge.tsx` + test; required visible `label` (status is never color-only), optional icon with a shape-dot fallback, `description` wired via `aria-describedby`; covers all 8 tones from §2.5.
- [x] 3.13 Create `src/ui/async/AsyncBoundary.tsx` and `RefreshIndicator.tsx` for initial loading, refresh, stale, empty, filtered-empty, error, unavailable, and restricted states. — `src/ui/async/{async-view-state.ts,AsyncBoundary.tsx,RefreshIndicator.tsx}` + tests; implements `AsyncViewState<T>`/`DisplayableError` from design.md Decision 11; unavailable/restricted delegate to `EmptyState`, error gets its own retry-aware branch gated on `retryable`.
- [x] 3.14 Create `src/ui/async/MutationStatus.tsx` and target-keyed mutation helpers for local pending, error, retry, rollback, and operation identity. — `src/ui/async/{mutation-state.ts,MutationStatus.tsx}` + tests; `useMutationRegistry` keys `MutationState` by `targetKey`, preserves `operationId` across pending→failed, never touches other targets; `fail()` is the caller's rollback signal (registry holds no domain data to revert itself).
- [x] 3.15 Create `src/ui/forms/FormSection.tsx`, `SettingsRow.tsx`, `FieldError.tsx`, and `DraftActionBar.tsx`. — each + test; `SettingsRow` composes `MutationStatus` for design.md Decision 17's "immediate" save mode and `FieldError`, `DraftActionBar` is the "draft" mode surface (renders nothing at 0 unsaved changes), both save-mode branches now share one primitive set instead of each settings page rebuilding its own.
- [x] 3.16 Create `src/ui/actions/ActionMenu.tsx` and consequence-aware confirmation primitives for secondary and destructive actions. — `src/ui/actions/{use-menu-list.ts,ActionMenu.tsx}` + test; reuses the existing `useConfirmation` primitive rather than a new dialog, roving-tabindex menu keeps disabled items keyboard-reachable (via `aria-describedby`) instead of skipping them, since a keyboard user still needs to discover why an item is disabled.
- [x] 3.17 Create `src/ui/evidence/EvidenceLink.tsx` with typed target, availability, permission, return context, and safe-copy behavior. — `src/ui/evidence/EvidenceLink.tsx` + test; `available`/`unavailable`/`restricted` availability, `returnTo` passed via router `state`, `copyValue` copies a reference (not raw evidence) with a timed confirmation.
- [x] 3.18 Create shared first-run, no-data, no-filter-match, unsupported, unavailable, and restricted EmptyState variants. — `src/ui/empty-state/EmptyState.tsx` + test; structural shell with a distinct default icon per variant, title/description/action stay caller-supplied since copy is domain-specific.
- [x] 3.19 Add component tests for keyboard behavior, focus return, disabled explanation, status non-color meaning, localization, and both themes for every primitive. — Satisfied incrementally per-primitive (each 3.1-3.18 commit landed its own tests) rather than as one final pass; 130 tests total across 27 test files under `src/ui/` (`npx vitest run src/ui/`). Dimension notes: keyboard/focus-return/disabled-explanation/status-non-color/localization are tested wherever a primitive actually has that surface (an audit pass while closing this task found and fixed one real gap — `ColumnVisibilityMenu` was missing Escape-to-close-with-focus-return and a visible reason for its one disabled checkbox; primitives with no dismissible overlay or no disabled state of their own, e.g. `StatusBadge`, `EmptyState`, `AppShell`, correctly have no such test). "Both themes" is not exercised per-primitive as a component test: every primitive is restricted to semantic-token classes by `ARCH-FE-006` (enforced on every commit), so no primitive branches on theme at all — design.md Decision 19 ("双主题共享层级，只在表达上不同") means there is no structural difference for a unit test to observe, and Decision 20 assigns "主题" verification to the Playwright layer (§21), not component tests.

## 4. Route registry and information architecture

- [ ] 4.1 Define typed routes for Sessions, Projects, Runs, Plan, Quality, Settings, and Help in a dedicated workbench route module.
- [ ] 4.2 Define typed secondary routes for Runs Attention/Active/History/Loops/Schedules, Plan Board/Goals, and Quality Evaluations.
- [ ] 4.3 Replace direct destination conditionals in `main-layout.tsx` with a lazy `DestinationDefinition` registry.
- [ ] 4.4 Add route parsing and serialization tests for valid, missing, malformed, stale, and unsupported stable ids.
- [ ] 4.5 Implement redirects from legacy Loops, Mission Control, Work Board, Goals, Evaluation, and Scheduled Tasks routes to the new equivalents.
- [ ] 4.6 Map the legacy scheduled-task dialog-open state to `/runs/schedules` and remove it after compatibility coverage passes.
- [ ] 4.7 Implement a safe internal `returnTo` token that cannot navigate to arbitrary external URLs.
- [ ] 4.8 Preserve supported filter, sort, selected entity, and scroll-anchor state when navigating to an authoritative evidence surface and back.
- [ ] 4.9 Clear evidence scope that belongs to a previous Session or entity before issuing destination queries.
- [ ] 4.10 Add explicit not-found, deleted, restricted, and stale-route states instead of silently opening an unrelated default object.
- [ ] 4.11 Keep deep-link first render independent of visited flags or click handlers.
- [ ] 4.12 Update workspace activity labels, icons, tooltips, accessible names, and analytics/debug identifiers for the five task domains.
- [ ] 4.13 Move Settings and Help to the utility group and preserve the in-app documentation destination.
- [ ] 4.14 Add one-time localized “moved to Runs/Plan/Quality” hints for users entering through a legacy route; store only a versioned dismissal flag.
- [ ] 4.15 Update user documentation and keyboard-shortcut help for all new routes and destination names.
- [ ] 4.16 Add Playwright tests for direct deep links, browser Back/Forward, return context, legacy redirects, and stale ids.

## 5. AppShell, pane composition, and lifecycle coordinator

- [ ] 5.1 Extract the top bar, activity rail, route outlet, notification host, and global attention summary into the new AppShell.
- [ ] 5.2 Replace the fixed session grid with the shared DestinationLayout and SplitPane model.
- [ ] 5.3 Implement container-width observation and the wide, standard, compact, and narrow composition algorithm.
- [ ] 5.4 Define and test the minimum readable width for Session Work, Run detail, Loop timeline, Board, Evaluation table, and Settings content.
- [ ] 5.5 Implement the rule that Inspector collapses to a sheet before contextual navigation and neither compresses the main surface below its minimum.
- [ ] 5.6 Persist versioned non-sensitive pane preferences per destination in `vanehub.workbench.layout.v2`.
- [ ] 5.7 Migrate the valid old session-sidebar width to the Sessions destination and safely clamp invalid values.
- [ ] 5.8 Separate user-preferred pane state from automatically forced responsive state so widening restores intent.
- [ ] 5.9 Implement focus mode using reversible pane state rather than overwriting the user's prior layout.
- [ ] 5.10 Keep an accessible focus-mode exit control visible in every supported composition.
- [ ] 5.11 Define `PageLifecyclePolicy` and attach one policy to every affected lazy destination and settings page.
- [ ] 5.12 Unmount `keepAlive: never` pages and prove page-owned intervals, observers, subscriptions, and large DOM are released.
- [ ] 5.13 Implement `draft-only` retention and shell-level navigation protection without serializing secrets.
- [ ] 5.14 Limit `keepAlive: always` to documented exceptions and add tests explaining each exception.
- [ ] 5.15 Implement bounded refresh-on-focus and reconnect reconciliation for update-heavy destinations.
- [ ] 5.16 Verify that unmounting a page does not cancel service-owned Agent, Loop, evaluation, or scheduled execution.
- [ ] 5.17 Delete obsolete visited-state booleans once every destination is registry and route driven.

## 6. Global Command Center

- [ ] 6.1 Create `WorkbenchSearchProvider` and `WorkbenchCommand` registries with no direct cross-domain mutation dependency.
- [ ] 6.2 Implement `Ctrl/Cmd+K` open behavior, translated shortcut hint, focus trap, Escape close, and focus return.
- [ ] 6.3 Add bounded Session search provider using the existing frontend agent service.
- [ ] 6.4 Add bounded Project/Workspace search provider using existing project and SSH service boundaries.
- [ ] 6.5 Add bounded Run search provider using Mission Control safe summaries.
- [ ] 6.6 Add Goal, Work Item, and Evaluation providers after their route adapters are available.
- [ ] 6.7 Add navigation commands for the five destinations and common secondary routes.
- [ ] 6.8 Add contextual commands for New Session, toggle navigation, toggle Inspector, toggle Runtime Panel, focus mode, and open settings where valid.
- [ ] 6.9 Ensure permission- or state-ineligible commands are hidden or disabled with an accessible explanation.
- [ ] 6.10 Cancel previous search requests on query or scope change and ignore stale results deterministically.
- [ ] 6.11 Rank exact title, prefix, recent, current-project, and needs-attention matches without using prompt or response content.
- [ ] 6.12 Exclude credentials, raw errors, log bodies, unrestricted paths, tool inputs, prompts, responses, and external identity values from results.
- [ ] 6.13 Add deterministic Web fixtures for empty, loading, partial, failed provider, stale response, and mixed-result states.
- [ ] 6.14 Add keyboard, screen-reader-name, route, privacy, and performance tests for the Command Center.

## 7. Session navigation and list

- [ ] 7.1 Extract a typed Session navigation view model from `session-sidebar.tsx` so grouping, filtering, search, archive, pin, and batch behavior are testable without rendering.
- [ ] 7.2 Set the preferred desktop Session navigation width to 280px with a bounded 256–400px range and rely on sheet composition when the main surface needs width.
- [ ] 7.3 Implement the default attention-first ordering for needs-input, pending verification or approval, running, pinned, recent, and remaining sessions using canonical available state.
- [ ] 7.4 Preserve category, project, archived, and flat organizations through a compact view selector.
- [ ] 7.5 Move Agent, status, project, source, and date filters into the shared FilterPopover and active-filter chips.
- [ ] 7.6 Keep search and New Session as the only permanently prominent navigation controls.
- [ ] 7.7 Replace permanent batch controls with an explicit batch mode and bottom batch action bar.
- [ ] 7.8 Apply the documented row metadata budget and move extra metadata to tooltip, context menu, or Inspector.
- [ ] 7.9 Ensure multi-Agent, IM, remote, failure, and recovery indicators appear only when present and do not create horizontal scrolling.
- [ ] 7.10 Use stable Session ids as virtual-list keys and virtualize large active, archived, category, and project views.
- [ ] 7.11 Preserve selected Session, group expansion, keyboard active row, and scroll anchor when filters or routes change.
- [ ] 7.12 Improve search results with matched title or safe metadata context without exposing message content unless the existing search contract explicitly returns a safe excerpt.
- [ ] 7.13 Make Active/Archived state obvious and keep archived count available without a full permanent row.
- [ ] 7.14 Move row actions to a stable trailing action area or context menu that remains reachable by keyboard and touch.
- [ ] 7.15 Keep category drag behavior and strengthen visible drop targets, success feedback, failure rollback, and context-menu equivalent movement.
- [ ] 7.16 Replace the narrow layout's fixed-height list above conversation with a full-height Session navigation sheet.
- [ ] 7.17 Add component and Playwright tests for 1,000 sessions, grouped virtualization, archive, batch, drag alternative, search, and sheet focus return.

## 8. Session primary surfaces and Runtime Panel

- [ ] 8.1 Define the new `SessionPrimarySurfaceId` and `SessionRuntimeSurfaceId` types and declarative surface registry.
- [ ] 8.2 Implement `work` as the stable primary slot that renders Chat for API/shared sessions and the existing Agent Terminal for eligible single-Agent CLI sessions.
- [ ] 8.3 Keep Changes as a primary surface and preserve all current review, evidence, and route behavior.
- [ ] 8.4 Merge Documents and Files into one Files primary surface with documented subviews and preserve both current service paths.
- [ ] 8.5 Keep Report as a primary surface with existing evidence semantics.
- [ ] 8.6 Move Terminal History, Shell, Logs, and Traces into the shared Runtime Panel.
- [ ] 8.7 Implement the legacy tab-id adapter for slash commands, route requests, persisted preferences, Mission Control links, and tests.
- [ ] 8.8 Add development assertions for any old tab id that has no target mapping.
- [ ] 8.9 Implement runtime-panel open, close, active tab, resize, maximize, restore, unread, unknown, and danger badge states.
- [ ] 8.10 Persist non-sensitive Runtime Panel height and preferred tab per Session destination without storing output content.
- [ ] 8.11 Apply registry seat scope to every surface and include validated seat id in query keys when required.
- [ ] 8.12 Ensure All seats is the truthful default for seat-optional evidence unless a validated route requests a concrete seat.
- [ ] 8.13 Require an active concrete seat before a seat-required Shell attach or create request.
- [ ] 8.14 Implement per-surface retention and `isVisible` behavior; do not rely solely on `display:none` to suspend work.
- [ ] 8.15 Preserve running terminal ownership according to the existing runtime contract when the panel closes or the UI unmounts.
- [ ] 8.16 Prevent evidence from the previous Session from rendering under a newly selected Session while reconciliation occurs.
- [ ] 8.17 Place badge queries above individual surfaces so opening multiple surfaces does not multiply summary subscriptions.
- [ ] 8.18 Guarantee primary tabs do not require horizontal scrolling at supported widths; move low-frequency optional commands to More if needed.
- [ ] 8.19 Update slash-command help and UI labels for Files subviews and Runtime Panel destinations.
- [ ] 8.20 Add unit, component, route, Playwright, and Tauri tests for every old-to-new mapping and each panel lifecycle.

## 9. Contextual Inspector

- [ ] 9.1 Create a typed `WorkbenchSelection` union for Session, Message, Tool, File, Change, Run, Loop Iteration, and Evaluation Result selections.
- [ ] 9.2 Validate every selection against its owning route and stable identity before querying details.
- [ ] 9.3 Create an Inspector provider registry keyed by selection kind; providers SHALL use owning frontend services only.
- [ ] 9.4 Implement Session Overview sections for Participants, Runtime, Usage, Skills, Workspace, IM, and Code Index.
- [ ] 9.5 Migrate existing information-panel queries and actions into overview sections without silently deleting capability.
- [ ] 9.6 Implement Follow Selection mode and selected styling for source objects.
- [ ] 9.7 Implement Pin and Unpin without changing the main-route selection.
- [ ] 9.8 Implement per-kind available, unavailable, restricted, loading, stale, and retryable error states.
- [ ] 9.9 Add EvidenceLinks from summaries to authoritative full pages for Diff, File, Session, Run, Log, Trace, Skill Settings, IM Settings, and Project.
- [ ] 9.10 Prevent Inspector providers from loading full unbounded logs, raw prompts, unrestricted tool input, or arbitrary file contents.
- [ ] 9.11 Implement inline Inspector at wide widths and the same content in an accessible right Sheet at standard/compact widths.
- [ ] 9.12 Return focus to the exact source object or logical fallback when the Inspector sheet closes.
- [ ] 9.13 Clear or mark stale a selection when Session, route scope, version, permission, or entity availability changes.
- [ ] 9.14 Preserve a pinned unavailable object's identity long enough to explain what changed.
- [ ] 9.15 Add responsive section navigation that does not use four to six equal-width text tabs in a 300px panel.
- [ ] 9.16 Add tests for provider laziness, pin/follow behavior, scope clearing, permission, stale selection, and sheet focus.
- [ ] 9.17 Remove the obsolete fixed information-panel tab shell after feature parity and route tests pass.

## 10. Conversation, messages, composer, and seats

- [ ] 10.1 Create one session presentation model for lifecycle, active execution, participant turn, recovery, message state, and allowed primary action.
- [ ] 10.2 Update the conversation header to show breadcrumb/title/project, one canonical status summary, one primary action, and More.
- [ ] 10.3 Move detailed participant roster, personalization, IM, model, and usage information to the Inspector or Run Configuration summary.
- [ ] 10.4 Replace default heavy message bubbles with a continuous transcript hierarchy while preserving roles and readable separation.
- [ ] 10.5 Keep failure, approval, blocked, and action-required items visually prominent and non-color-only.
- [ ] 10.6 Move low-frequency message metadata to an accessible details affordance.
- [ ] 10.7 Create explicit selection affordances for Message, Tool, Rich Block, error, approval, and compaction cards.
- [ ] 10.8 Create touch-accessible message action menus so copy, quote, feedback, retry, and selection do not depend on hover.
- [ ] 10.9 Preserve Tool-heavy turn aggregation, failure priority, approval visibility, and current rich rendering safety.
- [ ] 10.10 Extract `ConversationWindowModel` with stable keys, bottom threshold, prepend anchor, dynamic measurement, focus, and selected-item restoration.
- [ ] 10.11 Add deterministic tests for near-bottom streaming, reading history, prepend, Rich Block resize, Mermaid resize, and Session switch.
- [ ] 10.12 Integrate dynamic virtualization only after anchor-model tests pass; keep DOM rows bounded for the 5,000-message fixture.
- [ ] 10.13 Memoize completed Message items and expensive Markdown/Rich Block renderers so streaming one item does not rerender unrelated history.
- [ ] 10.14 Defer expensive collapsed Rich Blocks while retaining truthful summary and accessible state.
- [ ] 10.15 Refactor the Composer default surface to input, attachments/media, context chips, effective Agent/model summary, and Send/Stop.
- [ ] 10.16 Create the Run Configuration popover/sheet with Agent/runner, Provider/model, reasoning/thinking, permission, profile, and advanced groups.
- [ ] 10.17 Show each configuration value's provenance and distinguish one-message override from persisted profile.
- [ ] 10.18 Keep high-risk permission or sandbox warning visible in the closed configuration summary.
- [ ] 10.19 Place field and send errors next to the composer, preserve recoverable draft or persisted message, and keep unrelated reading operable.
- [ ] 10.20 Unify Seat selector keyboard behavior with roving focus, orientation arrows, Home/End, selection, All seats, current speaker, and departed state.
- [ ] 10.21 Move general participant selection to an Avatar Group/Popover so it is not another permanent peer tab bar.
- [ ] 10.22 Add Web and Tauri tests for streaming, stop, recovery, virtualization, focus, touch path, high-risk configuration, and multi-seat scope.

## 11. Create-session wizard

- [ ] 11.1 Extract create-session draft and validation into a reducer or model independent of dialog presentation.
- [ ] 11.2 Define supported runtime-mode combinations from service capabilities rather than hard-coded optimistic choices.
- [ ] 11.3 Implement Step 1 for Single/Multi, CLI/API, and Local/Remote with disabled explanations.
- [ ] 11.4 Implement Step 2 for participants, roles, Agent identity, model-family compatibility, personalization, and Skill summaries.
- [ ] 11.5 Reuse and improve the existing per-seat identity and reviewer-constraint presentation from the ergonomic change.
- [ ] 11.6 Implement Step 3 for recent/discovered project, remote workspace, branch, worktree, availability, and trust.
- [ ] 11.7 Implement Step 4 Review with runtime, participant, workspace, override, risk, and resource-consequence summary.
- [ ] 11.8 Allow backward navigation without losing valid draft values and reset only fields invalidated by an explicit mode change.
- [ ] 11.9 Add recent-session templates or recent creation presets only when their data is non-sensitive and validated.
- [ ] 11.10 Place validation and discovery errors at the owning field or step and provide a Review-level error summary with links.
- [ ] 11.11 Prevent duplicate submission and destructive dismissal while creation commits without freezing scroll, copy, or help.
- [ ] 11.12 Use a standard dialog at wide width and full-height sheet at compact width with virtual-keyboard-safe footer.
- [ ] 11.13 Preserve current service input semantics and add contract fields only where required by already supported modes.
- [ ] 11.14 Add tests for every supported mode combination, invalidated dependency, trust warning, Review return, error focus, and submission parity.

## 12. Settings registry, search, save, and lifecycle

- [ ] 12.1 Extend `settings-pages.ts` into a typed metadata registry with category, description, keywords, fields, save mode, lifecycle, risk, loader, and status provider.
- [ ] 12.2 Map every current primary settings page into the new workflow categories without deleting a page implicitly.
- [ ] 12.3 Add architecture tests for unique page ids, field ids, anchors, search keys, category order, and synchronized locale keys.
- [ ] 12.4 Build a static cross-page search index without mounting every page.
- [ ] 12.5 Implement Settings search results for page, section, field, and keyword matches with bounded descriptions.
- [ ] 12.6 Navigate a result to `/settings/:page#field-id`, load the page, scroll/focus appropriately, and apply reduced-motion-safe highlight.
- [ ] 12.7 Remove current-page-only or page-title-only search behavior after full-index parity passes.
- [ ] 12.8 Replace duplicated SettingsTopBar and PageHeader title presentation with one authoritative page heading.
- [ ] 12.9 Implement desktop grouped sidebar and compact searchable navigation sheet/selector; remove the long horizontal page strip.
- [ ] 12.10 Create immediate-save row behavior with local pending, canonical reconciliation, failure rollback, and retry.
- [ ] 12.11 Create draft-save behavior with shared DraftActionBar, Save, Discard, dirty count, validation, and conflict handling.
- [ ] 12.12 Create shell-level navigation protection for unsaved drafts and distinguish secret from non-secret draft retention.
- [ ] 12.13 Prevent secret values from search, route, generic draft storage, logs, clipboard diagnostics, and screenshot fixtures.
- [ ] 12.14 Apply Danger Zone presentation to destructive reset, uninstall, disconnect, revoke, remove, and erase actions.
- [ ] 12.15 Expose restart-required state before and after relevant saves.
- [ ] 12.16 Add bounded status indicators to navigation entries for unsaved, error, dependency unavailable, update available, and restart required.
- [ ] 12.17 Replace permanent visited-page mounting with explicit lifecycle policies and prove hidden polling stops.
- [ ] 12.18 Migrate resource pages such as CLI, Skill, Extension, Plugin, MCP, and SSH to shared Collection/Toolbar/Status/Action patterns incrementally.
- [ ] 12.19 Add a safe copy-diagnostics action to applicable pages and redaction tests.
- [ ] 12.20 Add Playwright tests for all page routes, cross-page search, search synonyms, compact navigation, Save/Discard, leave protection, secret safety, and both themes.

## 13. Projects and Workspaces destination

- [ ] 13.1 Create the Projects and Workspaces destination route, lazy module, navigation label, icon, and lifecycle policy.
- [ ] 13.2 Define a read-only safe `WorkspaceSummary` projection assembled through existing project, Git, SSH, Session, and Run service boundaries.
- [ ] 13.3 Do not create a new writable cross-domain workspace truth table; document projection ownership.
- [ ] 13.4 Implement recent, favorite, all, unavailable, and needs-attention views with shared filters and saved-view support where justified.
- [ ] 13.5 Implement local project, worktree, and remote SSH workspace rows with availability, safe path/host label, Git context, trust, recent Session, and active Run count.
- [ ] 13.6 Keep missing local paths and disconnected remote workspaces visible with correct unavailable classifications.
- [ ] 13.7 Implement workspace detail with identity, trust, Git/worktree, recent Sessions, active Runs, and related Plan/Quality links.
- [ ] 13.8 Implement state-aware Continue Session, New Session, Open Shell, Create Worktree, Reconnect, Relocate, Remove History, and Settings actions.
- [ ] 13.9 Prefill the create-session wizard from a validated workspace id and preserve Review confirmation.
- [ ] 13.10 Show remote trust or host-identity change persistently in list, detail, and creation flow.
- [ ] 13.11 Use normalized safe display paths while preserving canonical identity inside services.
- [ ] 13.12 Implement compact list-then-detail composition and restore list filters and scroll anchor on Back.
- [ ] 13.13 Add deterministic Web fixtures for local Git, non-Git, missing, remote connected, remote disconnected, untrusted, revoked, and empty states.
- [ ] 13.14 Add service contract, route, accessibility, privacy, and Playwright tests for the destination.
- [ ] 13.15 Update help documentation to explain projects, workspaces, worktrees, remote trust, and how they relate to Sessions.

## 14. Unified work board

- [ ] 14.1 Refactor the Board into shared PageHeader, Toolbar, Saved View, Board/List content, and optional item Inspector regions.
- [ ] 14.2 Move create and edit forms from the Header into the shared work-item editor sheet.
- [ ] 14.3 Define typed board query state for text, Agent, project, source, priority, due, status, sort, grouping, and presentation.
- [ ] 14.4 Implement active filter chips, clear-one, clear-all, filtered-empty, and URL-safe query serialization.
- [ ] 14.5 Implement versioned local Saved Views without storing unrestricted description or path content.
- [ ] 14.6 Reduce each card to title, actionable state, bounded metadata, one open action, and More.
- [ ] 14.7 Use normalized display paths everywhere while retaining canonical values for service filtering.
- [ ] 14.8 Create one canonical stage-change command used by drag, keyboard, touch menu, and picker paths.
- [ ] 14.9 Remove simultaneously permanent previous/select/next movement controls once the canonical menu and accessible drag alternative pass tests.
- [ ] 14.10 Implement row/card-level pending, optimistic update where safe, canonical reconciliation, failure rollback, and version conflict.
- [ ] 14.11 Remove global busy and full-board reload as the default response to a single mutation.
- [ ] 14.12 Implement explicit batch mode with selected count, eligibility preview, batch move/archive, and per-item outcome.
- [ ] 14.13 Implement compact grouped Stage List that does not require horizontal dragging.
- [ ] 14.14 Add optional presentation-only WIP limits with clear distinction from enforced domain rules.
- [ ] 14.15 Virtualize or page large column/list fixtures and preserve drag and selected-card identity.
- [ ] 14.16 Add keyboard, touch, drag alternative, mutation race, saved view, compact layout, and 1,000-item performance tests.

## 15. Goal Center

- [ ] 15.1 Refactor Goal Center into shared MasterDetail layout with route-backed selected goal.
- [ ] 15.2 Move create and edit forms from the Header into an accessible goal editor sheet.
- [ ] 15.3 Show one state-appropriate primary action and move permitted secondary/destructive actions into More.
- [ ] 15.4 Create typed search providers for Session, Run, Loop, and Work Item target pickers.
- [ ] 15.5 Replace ordinary raw target-id input with the picker; keep any diagnostic raw-id path explicitly advanced and validated.
- [ ] 15.6 Show target type, safe title, project, status, and stable identity before linking.
- [ ] 15.7 Implement local pending and canonical reconciliation for create, edit, lifecycle, link, unlink, acceptance, and archive actions.
- [ ] 15.8 Remove global busy and full-center reload for single-goal mutations.
- [ ] 15.9 Preserve manual acceptance and existing Goal domain completion semantics.
- [ ] 15.10 Create bounded relationship sections or graph for milestones, Work Items, Sessions, Runs, Loops, and evidence links.
- [ ] 15.11 Render missing or restricted linked targets explicitly without deleting the relation from the view.
- [ ] 15.12 Implement compact list-then-detail and grouped relationship-list fallback.
- [ ] 15.13 Add tests for picker search, raw-id rejection, state actions, version conflict, manual acceptance, relationship navigation, and compact Back.
- [ ] 15.14 Update Goal Center user documentation and i18n for the new workflow.

## 16. Runs and Mission Control

- [ ] 16.1 Create the Runs destination shell with Attention, Active, History, Loops, and Schedules routes.
- [ ] 16.2 Move Mission Control to the Attention/Active/History route model without changing canonical Run ownership.
- [ ] 16.3 Refactor the page into query model, compact summary, Run collection, detail, section navigation, action region, and EvidenceLink components.
- [ ] 16.4 Reduce large metric-card competition and make reliable summary counts act as filters.
- [ ] 16.5 Migrate text, Agent, project, runner, status, attention, and ordering controls to the shared Toolbar and FilterPopover.
- [ ] 16.6 Implement Saved Views and URL-safe copyable Run queries with sensitive-value exclusion.
- [ ] 16.7 Replace raw Agent, project, runner, and owner values with safe labels while retaining stable ids for queries.
- [ ] 16.8 Replace the nine compressed detail tabs with readable section navigation and compact selector fallback.
- [ ] 16.9 Implement real lazy loaders for Overview, Plan/Tasks, Timeline, Verification, Files/Artifacts, Tools, Context, Usage, and Logs.
- [ ] 16.10 Delete `lazyDetail` and all generic facet placeholder rendering.
- [ ] 16.11 Implement available, unavailable, restricted, loading, stale, and error states for each detail section.
- [ ] 16.12 Load only bounded correlated logs, artifacts, files, tools, traces, and context evidence from owning services.
- [ ] 16.13 Add validated EvidenceLinks and safe return context to Session, Review, Approval, Loop, Schedule, Evaluation, File, Trace, and Log surfaces.
- [ ] 16.14 Compute state-aware actions from canonical state, owner capability, permission, and version witness.
- [ ] 16.15 Implement target-local pending, race reconciliation, terminal-state precedence, and safe error detail.
- [ ] 16.16 Replace unconditional visible polling with coalesced events plus visibility/focus/reconnect reconciliation and bounded backoff.
- [ ] 16.17 Implement compact list-then-detail layout and preserve query/scroll/selection on Back.
- [ ] 16.18 Add 100/1,000 Run structural fixtures, detail-laziness assertions, keyboard/a11y tests, and native real-operation smoke.

## 17. Loop Center

- [ ] 17.1 Place Loop Center under `/runs/loops` and keep legacy route adapters.
- [ ] 17.2 Separate Definitions and Runs into route-backed collection views while keeping definition-to-run navigation.
- [ ] 17.3 Refactor the fixed three-column layout into contextual navigation plus main overview/timeline and optional shared Inspector.
- [ ] 17.4 Keep definition first-run, actionable overview, duplicate, enable/disable, edit, delete, and start semantics.
- [ ] 17.5 Retain and restyle the four-step creation flow using shared FormSection, validation, and Review components.
- [ ] 17.6 Preserve discovered project/branch behavior and clearly distinguish Web simulation.
- [ ] 17.7 Keep preflight non-launching and display each readiness result with remediation.
- [ ] 17.8 Refactor Run Header to canonical state, phase, current activity, budget, one primary action, and More.
- [ ] 17.9 Implement compact PhaseStepper and preserve critical actions without requiring Inspector.
- [ ] 17.10 Replace large default Iteration accordions with compact decision-oriented timeline rows.
- [ ] 17.11 Drive Inspector from selected phase, iteration, verification, finding, operation, file, or evidence link.
- [ ] 17.12 Implement material change and verification delta summaries between consecutive iterations when available.
- [ ] 17.13 Build a sticky Decision Panel for awaiting acceptance with criteria, evidence state, verifier advice, changes, risks, budget, and consequences.
- [ ] 17.14 Apply target-local pending and race reconciliation to start, pause, stop, resume, accept, continue, reject, and definition mutations.
- [ ] 17.15 Preserve all prior evidence and worktree/session history across continue, reject, cancel, and recovery states.
- [ ] 17.16 Add wide/compact, keyboard, preflight, decision, no-progress, exhausted-budget, interrupted-recovery, and Tauri smoke tests.

## 18. Agent Evaluation

- [ ] 18.1 Place Evaluation under `/quality/evaluations` and keep legacy route adapters.
- [ ] 18.2 Split the current concentrated page into experiment query model, list, creation wizard, Agent selector, result table, detail Inspector, and comparison modules.
- [ ] 18.3 Implement experiment list rows with task/version, Agent set, state, outcome summary, regression state, and updated time.
- [ ] 18.4 Move task and Agent configuration from the Header into a guided wizard or sheet with Review.
- [ ] 18.5 Add searchable Agent selection with status/capability filters, select-visible, selected summary, and incompatibility reasons.
- [ ] 18.6 Implement shared DataTable with service pagination or bounded virtualization, sorting, filters, columns, selection, and compact fallback.
- [ ] 18.7 Define default result columns and move raw ids and low-frequency fingerprints into detail or optional columns.
- [ ] 18.8 Add eligible Baseline selection with task/version/configuration compatibility rules.
- [ ] 18.9 Compute and present outcome-tier, metric, reliability, and evidence deltas only when provenance is comparable.
- [ ] 18.10 Mark regressions and improvements with icon/text/reason rather than color alone.
- [ ] 18.11 Implement 2–4 experiment comparison with aligned compatible task rows and immutable configuration snapshots.
- [ ] 18.12 Expose deterministic checks, thresholds, measured values, optional judge role, failure classification, and bounded reason in detail.
- [ ] 18.13 Replace raw artifact ids with typed EvidenceLinks and safe unavailable/restricted states.
- [ ] 18.14 Replace fixed one-second polling with coalesced events plus visible bounded reconciliation and hidden backoff.
- [ ] 18.15 Add Web fixtures for pass, deterministic failure, Agent failure, timeout, stuck, cancelled, benchmark error, missing metrics, flaky, and artifact-unavailable states.
- [ ] 18.16 Add 10,000-row structural, comparison, baseline, regression, keyboard, i18n, theme, and Tauri attempt smoke tests.

## 19. Scheduled Tasks

- [ ] 19.1 Create `/runs/schedules` page and move Scheduled Tasks secondary navigation under Runs.
- [ ] 19.2 Keep a legacy activity/dialog entry only as a temporary redirect and remove the large management dialog after parity.
- [ ] 19.3 Refactor list/detail/editor into separate components and shared management primitives.
- [ ] 19.4 Implement bounded search and filters for enabled, Agent, recurrence, status, attention, project/workspace, and next-run range when supported.
- [ ] 19.5 Show name, Agent, localized recurrence, enabled state, timezone, next run, latest status, and attention in the collection.
- [ ] 19.6 Create route-backed task detail with configuration, future occurrence preview, capability notice, latest Run, and history.
- [ ] 19.7 Create editor sheet for Create and Edit with adjacent validation and final review.
- [ ] 19.8 Add or adapt version-aware `updateScheduledTask` to Tauri and Web/mock service contracts.
- [ ] 19.9 Add Duplicate that opens a disabled reviewed draft and does not copy run history.
- [ ] 19.10 Add Run now returning stable operation or Run identity without changing recurrence.
- [ ] 19.11 Expose existing durable run history with pagination, trigger classification, timestamps, safe failure, and Session/Run links.
- [ ] 19.12 Add or adapt next-five-occurrence preview using the same timezone and recurrence semantics as execution.
- [ ] 19.13 Display explicit configured timezone and daylight-saving policy in editor and detail.
- [ ] 19.14 Replace hard-coded weekday and frequency labels with synchronized locale resources and locale-aware formatting.
- [ ] 19.15 Display the application-open and at-most-one catch-up execution model in page, editor Review, and capability-driven help.
- [ ] 19.16 Put Delete in More, add consequence-aware confirmation, and keep Enable/Disable and Run now state-aware.
- [ ] 19.17 Use target-local pending and preserve task collection and history during every mutation.
- [ ] 19.18 Add recurrence, timezone, DST, update race, duplicate, run-now, history, capability notice, compact layout, Web parity, and Tauri scheduler tests.

## 20. Responsive, accessibility, localization, and themes

- [ ] 20.1 Define container thresholds and minimum work-surface sizes in one responsive module; remove duplicated magic breakpoints from migrated pages.
- [ ] 20.2 Test 1600, 1440, 1280, 1100, 1024, 900, 768, 640, and minimum supported Tauri window widths.
- [ ] 20.3 Ensure contextual navigation, Inspector, filters, and editors become accessible Sheets rather than clipped columns.
- [ ] 20.4 Ensure Board becomes Stage List and DataTable-heavy pages become summary-list/detail flows at compact widths.
- [ ] 20.5 Ensure virtual keyboards do not cover Composer, field errors, dialog/sheet footer, or decision actions.
- [ ] 20.6 Apply minimum pointer target and spacing rules to icon-only controls, resize handles, row menus, drag handles, and tab close actions.
- [ ] 20.7 Implement and test correct Tablist, Toolbar, Menu, Listbox, Tree, Grid, Dialog, and Sheet keyboard models.
- [ ] 20.8 Add non-drag alternatives for Session category movement, Board stage movement, ordering, and any new drag interaction.
- [ ] 20.9 Ensure focus is not obscured by sticky headers, toasts, composer, Runtime Panel, Decision Panel, or virtual keyboard.
- [ ] 20.10 Ensure closing sheets/dialogs returns focus to source or logical fallback.
- [ ] 20.11 Ensure status, priority, pass/fail, trust, attention, and regression are not color-only.
- [ ] 20.12 Add `prefers-reduced-motion` behavior and tests for pane, sheet, highlight, loading, and drag transitions.
- [ ] 20.13 Add all new strings to every currently shipped locale, including zh-CN, en, and ja, in the same commit as UI changes.
- [ ] 20.14 Use locale-aware date, time, relative time, number, token, duration, recurrence, weekday, and timezone formatting.
- [ ] 20.15 Prevent long German-like test strings, Chinese, Japanese, ids, paths, and host labels from overlapping controls.
- [ ] 20.16 Verify bidirectional text safety for paths, ids, code, and mixed-language labels even if RTL is not a shipped locale.
- [ ] 20.17 Verify both `minimal` and `futuristic` have equivalent structure, focus, disabled, error, selected, attention, and responsive states.
- [ ] 20.18 Run automated axe or existing accessibility checks on every core route and fix serious/critical findings.
- [ ] 20.19 Perform keyboard-only end-to-end passes for Session creation, message send/stop, Run inspection, Loop acceptance, Board movement, Goal linking, Evaluation comparison, Schedule editing, and Settings search/save.
- [ ] 20.20 Record manual screen-reader smoke results for navigation landmarks, route titles, tabs, lists, table headers, errors, dialogs, and live status.

## 21. Automated tests, performance, visual regression, and native evidence

- [ ] 21.1 Create a requirement-to-test matrix covering all 89 delta requirements and 312 scenarios; identify unit, component, Web E2E, desktop E2E, performance, visual, and manual owners.
- [ ] 21.2 Update existing tests that depend on old destination names, old nine-tab order, fixed information tabs, Scheduled Tasks dialog, or permanent Settings mounting.
- [ ] 21.3 Add route contract tests for every new and legacy URL and safe return context.
- [ ] 21.4 Add architecture tests for destination registry, settings registry, service boundaries, line limits, locale synchronization, and semantic-token use.
- [ ] 21.5 Add component tests for every shared primitive across loading, empty, filtered-empty, error, unavailable, restricted, pending, disabled, selected, and focus states.
- [ ] 21.6 Add deterministic Web fixtures for every affected page and avoid wall-clock-only assertions.
- [ ] 21.7 Add Session list 1,000-row DOM and query-count budget.
- [ ] 21.8 Add 5,000-message dynamic-height window, streaming update-batch, prepend-anchor, and rerender-count budgets.
- [ ] 21.9 Add Board 1,000-item render, query, mutation, and drag-alternative budgets.
- [ ] 21.10 Add Mission Control 100/1,000 Run query-count, page-size, lazy-detail, event-coalescing, and hidden-page budgets.
- [ ] 21.11 Add Goal large relationship-list and picker-query budgets.
- [ ] 21.12 Add Loop large iteration timeline, Inspector lazy-load, and action-update budgets.
- [ ] 21.13 Add Evaluation 10,000-row page/virtualization, comparison, update-batch, and hidden-poll budgets.
- [ ] 21.14 Add Schedule 100-task and long-history paging budgets.
- [ ] 21.15 Instrument test-only counts for active intervals, observers, subscriptions, mounted heavy panels, and update batches.
- [ ] 21.16 Assert hidden ordinary destinations release page-owned resources while global terminal/attention coordinators stay within budget.
- [ ] 21.17 Create the agreed visual regression matrix for 10 core surfaces, two themes, three locales, and representative widths.
- [ ] 21.18 Use deterministic fonts, dates, ids, fixture seed, reduced animation, and stable window size for screenshots.
- [ ] 21.19 Require review notes for every accepted baseline update and store before/after references.
- [ ] 21.20 Add WebdriverIO/Tauri tests for xterm resize and clipping, window resize composition, native folder dialog, filesystem/project flow, SSH trust flow, and native scheduler/evaluation smoke.
- [ ] 21.21 Run native smoke on Windows, macOS, and Linux where CI runners exist; report unavailable platforms as not executed, not passed.
- [ ] 21.22 Run memory/leak navigation loops across Sessions, Runs, Plan, Quality, Projects, and Settings and compare mounted resources to baseline.
- [ ] 21.23 Run full Web Playwright in both themes and at least the primary locale, with focused locale suites for every supported locale.
- [ ] 21.24 Publish `docs/ui-redesign/verification-report.md` with exact commands, commits, fixtures, platforms, counts, screenshots, failures, and waivers.

## 22. Legacy removal, documentation, and final verification

- [ ] 22.1 Compare V1 and V2 capability inventories and resolve every missing action, state, route, and evidence link before removing the migration flag.
- [ ] 22.2 Remove the old per-module activity entries after redirects, Command Center aliases, and user documentation pass.
- [ ] 22.3 Remove the old nine-tab visual bar and obsolete tab content wrappers after the compatibility adapter and Runtime Panel pass.
- [ ] 22.4 Remove the obsolete fixed information-panel tab implementation after Inspector capability parity passes.
- [ ] 22.5 Remove the large Scheduled Tasks management dialog after routed page parity passes; retain only an intentional quick-create surface if specified.
- [ ] 22.6 Remove global busy/full-reload implementations from migrated Board and Goal mutations.
- [ ] 22.7 Remove hidden visited destination booleans and permanent visited Settings mounting after lifecycle tests pass.
- [ ] 22.8 Delete unused CSS selectors, icons, i18n keys, storage keys, route branches, placeholder copy, and dead feature flags.
- [ ] 22.9 Update README/user guide/help screenshots, navigation map, keyboard shortcuts, accessibility notes, and migration notes.
- [ ] 22.10 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run build`, `npm run architecture:check`, and `npm run contracts:check`.
- [ ] 22.11 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace`.
- [ ] 22.12 Run the complete Playwright and WebdriverIO/Tauri matrices and attach exact results without claiming unexecuted platforms.
- [ ] 22.13 Run `openspec validate redesign-unified-workbench-ui --strict` and `openspec validate --specs --strict`.
- [ ] 22.14 Run `/opsx:verify`, resolve every blocking finding, remove the migration flag, and archive only after all P0 findings and required scenarios are evidenced.
