# Implementation notes: `redesign-unified-workbench-ui`

Task 1.2. Adapts the delivered `IMPLEMENTATION_GUIDE.md`'s milestone breakdown (outside this repo,
under the delivery package root) into a tracked repo artifact, corrected against what Milestone 0
actually verified rather than copied blind — see `docs/ui-redesign/baseline.md` for the evidence
behind every correction noted below.

## Milestone ownership and dependency order

```
M0 Baseline & contracts (§0, §1)
 └─▶ M1 Design system & shell (§2–§6)
      ├─▶ M2 Sessions (§7–§11)
      │     └─▶ M4 Quality, Settings, Projects (§12, §13, §18)
      └─▶ M3 Runs & Plan (§14–§17, §19)
            └─▶ M4 Quality, Settings, Projects (§12, §13, §18)
                  └─▶ M5 Stabilization & legacy removal (§20–§22)
```

- **M0** (done, `1cbaba82`): repository baseline, dependency check, runtime-verified inventory,
  fixtures, screenshots. No application code changed.
- **M1** (§2 tokens, §3 shared primitives, §4 route registry, §5 AppShell/pane/lifecycle, §6
  command center): builds the shell `M2` and `M3` both depend on. Nothing here may import a
  feature service (`ARCH-FE-005`, task 1.4) or call Tauri `invoke()` outside the adapter naming
  convention (`ARCH-FE-001`, broadened in task 1.5 to close the `.ts`-hook-file gap the original
  `.tsx`-only check missed).
- **M2** (§7 session nav, §8 session surfaces, §9 Inspector, §10 conversation/composer, §11 create
  wizard) and **M3** (§14 Board, §15 Goals, §16 Runs/Mission Control, §17 Loop, §19 Scheduled
  Tasks) can proceed in parallel once M1's shell exists — they consume the same registries but own
  disjoint feature directories (confirmed via the `UI_PRIMITIVE_FORBIDDEN_ROOTS` denylist in
  `scripts/architecture/frontend-rules.mjs`).
- **M4** (§12 Settings, §13 Projects & Workspaces, §18 Evaluation) depends on both M2 and M3
  because Settings' cross-page search indexes fields from pages built in both, Projects prefills
  the M2 create-session wizard, and Evaluation's Agent selector composes with M2's Agent/model
  concepts.
- **M5** (§20 responsive/a11y/i18n, §21 tests/perf/visual/native, §22 legacy removal) is the only
  milestone allowed to delete the `unifiedWorkbenchV2` flag (task 1.1) and the pre-redesign shell —
  everything before it must keep both live side by side.

## Corrections Milestone 0 made to the delivered planning documents

These are the load-bearing facts later milestones must build against, not the delivery package's
original (reasonable but unverified) assumptions:

1. **Routing**: `react-router` genuinely handles the 3 top-level routes, but `/workspace/*`
   destinations are deliberately hand-parsed rather than nested `<Route>` elements, specifically
   because Route unmounts on navigation and the app requires previously-visited destinations to
   stay mounted. The new `DestinationDefinition` registry (§4) must preserve that mount-retention
   property, not just wrap the existing behavior in a more route-idiomatic shape that would
   reintroduce unmounting.
2. **i18n surface**: 5 registered locales (`zh-CN`, `en`, `zh-TW`, `ja`, `ko`), not the 3
   (`zh-CN`, `en`, `ja`) `tasks.md`'s §20.13/20.17 name as examples. New strings go in all 5, per
   `AGENTS.md`'s existing "every registered locale" rule, which controls over the incomplete
   example list.
3. **"Runs" has no unifying concept today** — it is `AgentRun` (`mission-control-service.ts`),
   `ExecutionRun` (`execution-observability-service.ts`), `OperationTask` (`operation-service.ts`),
   and `ExecutionRecord` (`session-workspace-evidence-service.ts`). §16/§18 must compose a UI-level
   query model across these, per the change's own Non-Goal against merging them into one DB model.
4. **Mission Control → Session cross-destination navigation is confirmed partially implemented**:
   only `kind === "review"` opens a matching tab; `loop`/`goal`/`evaluation` kinds just navigate to
   the session. This is concrete runtime evidence behind the static audit's "Mission Control has
   placeholder facets" claim, not merely an assumption — real work for §16.13/§9.

## Deprecation policy (task 1.7)

Documented at the point of use in `src/lib/legacy-id-diagnostics.ts` (also has the
`warnUnmappedLegacyId` dev-only diagnostic that §4/§8's compatibility adapters must call for an
unrecognized legacy destination or session-tab id). Summary: legacy ids survive through a
compatibility adapter for one full stable release cycle after §4/§8 ship, matching `design.md`'s
"旧 route adapter 至少保留一个稳定版本周期" commitment; removal is §22.1's decision, not something
an unrelated later task may do quietly.

## Process commitments (tasks 1.8–1.9)

These are not code — they are constraints on how I (and anyone continuing this change) work:

- **Task 1.8**: each milestone's commit(s) must pass their focused tests and
  `openspec validate redesign-unified-workbench-ui --strict` before the next milestone starts.
  Applied so far: M0 was fully committed and evidenced before any M1 code was written.
- **Task 1.9**: a task whose behavior depends on xterm, WebView, native dialogs, filesystem
  selection, or SSH is not checked off from browser-only (Playwright/Vitest) evidence — it needs
  the corresponding `test:desktop:*` layer, and if that layer can't be run to completion in the
  current environment (as happened with task 0.6), the checkbox records that honestly rather than
  claiming a pass that never happened.

## Spec-delta reconciliation against the current stable specs

The delivered spec deltas (`specs/*/spec.md`) initially failed `openspec validate --strict`: 15
`MODIFIED` requirements across 6 files (`main-layout-ui` ×6, `loop-management-ui` ×6,
`agent-mission-control` ×4, `chat-experience` ×1, `scheduled-task-management` ×1,
`settings-center-ui` ×2) omitted ~70 scenarios that the current stable specs
(`openspec/specs/<capability>/spec.md`) still have — because a `MODIFIED` block replaces the whole
requirement, archiving would have silently dropped that coverage. All 15 have been reconciled; the
change now validates cleanly (`openspec validate redesign-unified-workbench-ui --strict` passes).

**The validator matches scenarios by title only, not content** (confirmed empirically, not
documented anywhere found). This means an old scenario whose underlying behavior the redesign
genuinely changes can keep its original title as a stable "slot" while its body is rewritten to
describe the new behavior — which is what most of the 70 reconciled scenarios do, since many
described navigation paths, layouts, or mount-lifecycle behavior the redesign deliberately changes
(e.g. "Open Loops from activity bar" now redirects to the new Runs secondary route instead of
asserting Loops is a primary activity-bar entry; "Return to a visited lazy page" now describes
`keepAlive: never` unmounting instead of the old permanent-mount behavior it replaces). A genuinely
still-valid old scenario was copied forward with only terminology updates (e.g. "tab" → "surface" /
"section" where the registry model renamed the concept). If a future task edits any of these 6
files' `MODIFIED` requirements further, re-run `openspec validate redesign-unified-workbench-ui
--strict` before considering the edit done — dropping a scenario title silently reintroduces this
class of error, and it will not surface until archive time otherwise.

## Requirement traceability

See `traceability.md` in this directory for the full 89-requirement-to-task mapping.
