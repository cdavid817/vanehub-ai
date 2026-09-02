import type { AgentRunState } from "../types/agent-run";
import type { MissionControlCounts, MissionControlQuery, MissionControlSort } from "../types/mission-control";

/**
 * Task 16.3's own "query model" extraction (design.md Decision 11) -- filters, sort, and cursor,
 * previously five separate `useState` calls directly inside mission-control.tsx.
 *
 * `states` is an array, not the single value the old `status` field held, even though the status
 * `<select>` still only ever picks one state at a time -- see `MISSION_CONTROL_COUNT_STATES`'s own
 * doc comment for why an exact mapping from the summary counts (16.4) requires it.
 */
export interface MissionControlFilterState {
  agentId: string;
  projectId: string;
  states: AgentRunState[];
  runner: "" | "local" | "ssh";
  sort: MissionControlSort;
}

export const defaultMissionControlFilterState: MissionControlFilterState = {
  agentId: "", projectId: "", states: [], runner: "", sort: "attention",
};

/**
 * The pre-existing status `<select>`'s own option set (mission-control.tsx's old `states` const),
 * migrated verbatim into the new FilterPopover field -- not the full `AgentRunState` union.
 * "created"/"preparing"/"paused"/"verifying"/"cancelled" were never offered before this pass either
 * (transient or rare states a reader is unlikely to filter by); "blocked" is deliberately absent
 * too, for the same reason it is not a plain `<option>` here -- see `MISSION_CONTROL_COUNT_STATES`.
 */
export const MISSION_CONTROL_STATUS_OPTIONS: AgentRunState[] = [
  "running", "waiting_approval", "waiting_user", "retrying", "stuck", "failed", "completed",
];

/** Every `AgentRunState` this app's own UI can ever place into `states` -- the 7 plain dropdown
 *  options above plus "blocked" (reachable only via the count-card mapping below, not a dropdown
 *  option in its own right). Used to validate persisted filter/saved-view state, not to render
 *  options. */
export const MISSION_CONTROL_REACHABLE_STATES: AgentRunState[] = [...MISSION_CONTROL_STATUS_OPTIONS, "blocked"];

/**
 * 16.4's own "confirm the mapping is exact, don't guess": every `MissionControlCounts` field maps
 * to the `AgentRunState` value(s) that produce it server-side. Confirmed by reading both backends'
 * own count projection (`overview()` in mission_control.rs and `webMissionOverview` in
 * web-mission-control-client.ts), which agree byte-for-byte:
 *
 * ```
 * blocked: count("blocked") + count("stuck")
 * ```
 *
 * `blocked` is the one exception to "one count, one state" -- it is a union of two distinct
 * `AgentRunState` values, not a single one. Collapsing it to just `"stuck"` (the only one of the
 * pair already offered by the plain status dropdown above) would under-filter: the metric shown on
 * the card would count more runs than clicking it would ever display. This is why
 * `MissionControlFilterState.states` is an array -- a single-value field has no honest value to
 * hold for this one card.
 */
export const MISSION_CONTROL_COUNT_STATES: Record<keyof MissionControlCounts, AgentRunState[]> = {
  running: ["running"],
  waitingApproval: ["waiting_approval"],
  waitingUser: ["waiting_user"],
  retrying: ["retrying"],
  blocked: ["blocked", "stuck"],
  failed: ["failed"],
  completedRecently: ["completed"],
};

/** Order-independent set equality -- used to tell whether the currently active `states` filter is
 *  exactly one count card's own mapping (so that card alone renders pressed), not just non-empty. */
export function sameStateSet(a: readonly AgentRunState[], b: readonly AgentRunState[]): boolean {
  if (a.length !== b.length) return false;
  const set = new Set(a);
  return b.every((state) => set.has(state));
}

/** Narrowing filters only -- `sort` changes display order over the same result set, not what is in
 *  it (mirrors `isWorkBoardFilterActive`'s own text/sort split in work-board-query.ts). */
export function isMissionControlFilterActive(filter: MissionControlFilterState): boolean {
  return Boolean(filter.agentId) || Boolean(filter.projectId) || filter.states.length > 0 || Boolean(filter.runner);
}

/** Clears every narrowing dimension but leaves `sort` untouched -- mirrors `work-board.tsx`'s own
 *  `clearFilters`, which likewise never resets its own display-order field. */
export function clearMissionControlFilters(filter: MissionControlFilterState): MissionControlFilterState {
  return { ...filter, agentId: "", projectId: "", states: [], runner: "" };
}

/** Projects the UI-facing filter state into the shape `agentService.getMissionControlOverview`
 *  understands -- an empty string means "no filter" for every string field, mirroring the
 *  pre-existing `agentId || undefined` pattern this replaces. */
export function toMissionControlQuery(filter: MissionControlFilterState, cursor: string | null): MissionControlQuery {
  return {
    agentId: filter.agentId || undefined,
    cursor,
    limit: 20,
    projectId: filter.projectId || undefined,
    runner: filter.runner || undefined,
    sort: filter.sort,
    states: filter.states.length ? filter.states : undefined,
  };
}
