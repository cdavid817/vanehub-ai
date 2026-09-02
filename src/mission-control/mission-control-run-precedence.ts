import type { AgentRunState } from "../types/agent-run";
import type { MissionControlOverview, MissionControlPage, MissionControlRunSummary } from "../types/mission-control";

/**
 * Mirrors `RunState::is_terminal()` (src-tauri/src/contexts/operations/domain/run.rs) and the Web
 * mock's own `terminalRunStates` (services/web-agent-run-state.ts) -- both backends agree on this
 * exact three-state set, confirmed by reading both. A fixed fact about the `AgentRunState` enum,
 * not a predicted/derived value, so it is safe to hardcode client-side for task 16.15's own
 * "terminal-state precedence".
 */
export function isTerminalMissionControlRunState(state: AgentRunState): boolean {
  return state === "completed" || state === "failed" || state === "cancelled";
}

/**
 * Picks whichever of `existing`/`incoming` is the more authoritative reading of one Run, for
 * merging a background/racing update against what is already shown on screen.
 *
 * Two rules, in order: (1) once a run is known terminal, a later read reporting it as *not*
 * terminal is always stale -- terminal is final in this domain (no backend ever advances a run's
 * version again after Completed/Failed/Cancelled; confirmed in both `updateWebAgentRun` and
 * `run_service.rs`), so `existing` wins outright regardless of version bookkeeping. (2) Otherwise
 * the higher `version` wins -- `version` is the optimistic-concurrency witness every mutation
 * already threads through (`performMissionControlAction`'s own `version` field), so it doubles as
 * an ordering token: a response carrying an older version is, by definition, describing an earlier
 * moment than one already applied, however the two responses happened to arrive.
 */
export function preferAuthoritativeRun(
  existing: MissionControlRunSummary | undefined,
  incoming: MissionControlRunSummary,
): MissionControlRunSummary {
  if (!existing) return incoming;
  if (isTerminalMissionControlRunState(existing.state) && !isTerminalMissionControlRunState(incoming.state)) return existing;
  return incoming.version >= existing.version ? incoming : existing;
}

function findKnownRun(overview: MissionControlOverview | null, runId: string): MissionControlRunSummary | undefined {
  if (!overview) return undefined;
  return overview.attention.items.find((item) => item.runId === runId)
    ?? overview.active.items.find((item) => item.runId === runId)
    ?? overview.recent.items.find((item) => item.runId === runId);
}

/**
 * Merges a freshly-fetched `MissionControlOverview` (the 2-second poll, a focus/visibility
 * reconcile, or a manual refresh -- every caller of `load()` in mission-control.tsx) against
 * whatever is already known, so a slow poll response that happens to land after a faster mutation
 * response cannot regress a run back to a stale, pre-mutation reading (16.15's race-reconciliation
 * and terminal-state-precedence requirements). Section membership (which of attention/active/recent
 * a run belongs to) is left entirely to the server's own `incoming` categorization here -- only the
 * per-run fields are guarded, via `preferAuthoritativeRun`.
 */
export function mergeMissionControlOverview(
  previous: MissionControlOverview | null,
  incoming: MissionControlOverview,
): MissionControlOverview {
  const mergeSection = (page: MissionControlPage): MissionControlPage => ({
    ...page,
    items: page.items.map((item) => preferAuthoritativeRun(findKnownRun(previous, item.runId), item)),
  });
  return {
    counts: incoming.counts,
    attention: mergeSection(incoming.attention),
    active: mergeSection(incoming.active),
    recent: mergeSection(incoming.recent),
  };
}

function belongsInSection(section: "attention" | "active" | "recent", run: MissionControlRunSummary): boolean {
  if (section === "attention") return run.attention !== null;
  const terminal = isTerminalMissionControlRunState(run.state);
  return section === "recent" ? terminal : !terminal;
}

function reconcileSection(page: MissionControlPage, fresh: MissionControlRunSummary, belongs: boolean): MissionControlPage {
  const existing = page.items.find((item) => item.runId === fresh.runId);
  if (!existing) return page; // Never insert into a page the client has not already fetched -- see module doc below.
  if (!belongs) return { ...page, items: page.items.filter((item) => item.runId !== fresh.runId) };
  return { ...page, items: page.items.map((item) => (item.runId === fresh.runId ? preferAuthoritativeRun(existing, fresh) : item)) };
}

/**
 * Patches one Run's own fresh summary (a mutation's own receipt, or a version-conflict refetch)
 * into whichever of attention/active/recent currently contains it, in place, and drops it from any
 * section it no longer belongs to. Never *adds* a run to a section it was not already showing in --
 * the three sections are independently paginated and sorted server-side (see `mission_control.rs`'s
 * own `scoped()` / the Web mock's own `webMissionOverview`), so fabricating where in that order a
 * newly-qualifying run would land is exactly the kind of guess `use-goal-center-actions.ts` already
 * documents declining to make for server-derived fields. A run that moves out of every section it
 * was in simply drops from view until the next natural `load()` -- the same tradeoff
 * `use-work-board-actions.ts`'s `mutateCard` accepts for a card whose mutation moves it out of the
 * current archived/active scope. `counts` is left untouched for the same reason (it is a
 * whole-dataset aggregate, not something derivable from one changed run) and is corrected by the
 * next natural `load()`.
 */
export function patchMissionControlRun(overview: MissionControlOverview, fresh: MissionControlRunSummary): MissionControlOverview {
  return {
    counts: overview.counts,
    attention: reconcileSection(overview.attention, fresh, belongsInSection("attention", fresh)),
    active: reconcileSection(overview.active, fresh, belongsInSection("active", fresh)),
    recent: reconcileSection(overview.recent, fresh, belongsInSection("recent", fresh)),
  };
}
