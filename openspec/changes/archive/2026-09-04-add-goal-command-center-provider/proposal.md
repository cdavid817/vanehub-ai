## Why

`PlanDestination` now consumes `goalId` from the URL (task 15.1 of the now-archived
`redesign-unified-workbench-ui` change), which is exactly the precondition task 6.6 of that change
named for adding a Goal search provider to the Global Command Center — deliberately deferred at the
time because no route adapter existed yet to land on. That precondition is now met and independently
re-verified: `plan-destination.tsx` wires `location.goalId` into `GoalCenter` as its initial
selection and reports changes back via `onSelectGoal`. Users can already search Sessions, Projects,
and Runs from `Ctrl/Cmd+K`; Goals are the one domain with a working target to land on that still has
no search entry point.

## What Changes

- Add a `GoalSearchProvider`, mirroring the existing `SessionSearchProvider`/`ProjectSearchProvider`/
  `RunSearchProvider` pattern (`session-search-provider.ts`, `project-search-provider.ts`,
  `run-search-provider.ts`) — same `WorkbenchSearchProvider` interface, no direct cross-domain
  mutation dependency.
- Register it in `command-center-registry.ts`'s `SEARCH_PROVIDERS` aggregation, and update that
  file's own doc comment (which currently documents 6.6's deferral) to reflect the new state.
- Selecting a Goal search result navigates to the existing `PlanSection`/`goalId` route
  (`/workspace/plan/goals/<goalId>`), the same navigation shape the Run provider already uses.
- Explicitly **not** in scope: Work Item and Evaluation providers. Both remain genuinely blocked —
  `WorkBoard` still has no injectable initial-selection prop for `workItemId`, and
  `EvaluationCenter`'s "selected" concept (a run attempt) does not map cleanly onto "experiment"
  without its own separate design decision. Neither is attempted here.

## Capabilities

### New Capabilities
- `global-command-center`: First formal spec requirement for the Ctrl/Cmd+K Global Command Center,
  scoped to the Goal search provider being added here. The Command Center shell and its existing
  Session/Project/Run providers already shipped in `redesign-unified-workbench-ui` without ever
  being captured as a formal spec Requirement (confirmed: zero matches for "Command Center" across
  every file in `openspec/specs/`) — a pre-existing gap this change does not attempt to backfill.
  This capability starts scoped to only the Goal provider; Session/Project/Run remain undocumented
  at the spec level until a future change chooses to close that gap.

### Modified Capabilities
(none — `goal-management`'s own requirements are unchanged; this only adds a new way to reach an
existing Goal, not a new Goal capability)

## Impact

- **Frontend only**, both Web and Desktop runtime (the Command Center is a shared, adapter-agnostic
  UI feature triggered client-side — no Tauri-specific behavior, no new Rust command).
- New file: `src/command-center/goal-search-provider.ts`, wrapping the same `goalService` boundary
  `GoalCenter` already uses — no new service-layer surface.
- Modified: `src/command-center/command-center-registry.ts` (register the new provider; revise its
  doc comment).
- No impact to frontend/backend isolation or runtime adapter boundaries: the new provider depends
  only on the existing `src/services/agent-service.ts`-boundary `goalService`, the same pattern the
  three existing providers already follow.
