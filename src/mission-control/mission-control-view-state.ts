import { MISSION_CONTROL_REACHABLE_STATES, type MissionControlFilterState } from "./mission-control-query";

/**
 * 4.8: "preserve filter, sort, ... state when navigating to an authoritative evidence surface and
 * back." `sessionStorage`, not `localStorage` -- this is the reader's current working filter, not a
 * durable preference; a fresh app launch days later should not silently re-apply a stale runner
 * filter the reader has long forgotten setting.
 *
 * Reuses `MissionControlFilterState` verbatim rather than a parallel, structurally-identical
 * interface -- the two used to differ (this held a single `status` string; 16.4's own exact
 * count-to-filter mapping required the query model to switch to a `states` array, see
 * mission-control-query.ts's own doc comment), and keeping one shared type means they cannot
 * silently drift apart again. The storage key bumped from `.v1` to `.v2` for that same schema
 * change, so an old single-`status` blob from a build before this pass is discarded rather than
 * misread.
 */
export type MissionControlViewState = MissionControlFilterState;

const VIEW_STATE_KEY = "vanehub.mission-control.view.v2";
const SCROLL_KEY = "vanehub.mission-control.scroll.v1";

function isValidStates(value: unknown): value is MissionControlFilterState["states"] {
  return Array.isArray(value) && value.every((item) => (MISSION_CONTROL_REACHABLE_STATES as string[]).includes(item as string));
}

export function readMissionControlViewState(): MissionControlViewState | null {
  if (typeof sessionStorage === "undefined") return null;
  try {
    const raw = sessionStorage.getItem(VIEW_STATE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<MissionControlViewState>;
    if (typeof parsed.agentId !== "string" || typeof parsed.projectId !== "string" || !isValidStates(parsed.states)) return null;
    if (parsed.runner !== "" && parsed.runner !== "local" && parsed.runner !== "ssh") return null;
    if (parsed.sort !== "newest" && parsed.sort !== "oldest" && parsed.sort !== "attention") return null;
    return { agentId: parsed.agentId, projectId: parsed.projectId, states: parsed.states, runner: parsed.runner, sort: parsed.sort };
  } catch {
    return null;
  }
}

export function writeMissionControlViewState(state: MissionControlViewState): void {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.setItem(VIEW_STATE_KEY, JSON.stringify(state));
}

export function readMissionControlScrollTop(): number {
  if (typeof sessionStorage === "undefined") return 0;
  const raw = sessionStorage.getItem(SCROLL_KEY);
  const value = raw ? Number(raw) : 0;
  return Number.isFinite(value) && value >= 0 ? value : 0;
}

export function writeMissionControlScrollTop(scrollTop: number): void {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.setItem(SCROLL_KEY, String(scrollTop));
}
