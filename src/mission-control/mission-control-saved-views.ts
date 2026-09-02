import type { AgentRunState } from "../types/agent-run";
import type { MissionControlSort } from "../types/mission-control";
import { MISSION_CONTROL_REACHABLE_STATES, type MissionControlFilterState } from "./mission-control-query";

export const MISSION_CONTROL_SAVED_VIEW_NAME_MAX_LENGTH = 60;

const runnerOptions = ["", "local", "ssh"] as const;
const sortOptions: MissionControlSort[] = ["newest", "oldest", "attention"];

/**
 * 16.6: a new, separate, versioned, named-list mechanism layered on top of the pre-existing
 * ephemeral `mission-control-view-state.ts` (which stays exactly as-is for "last used" recall) --
 * mirrors `work-board-saved-views.ts`'s own shape (14.5) rather than extending the ephemeral module,
 * because a Saved View is a durable, user-named, user-managed list entry (create/apply/delete), a
 * fundamentally different lifecycle from "whatever was active when the reader last left the page".
 * Folding both into one module would conflate a single auto-updating blob with a list a reader
 * explicitly curates.
 *
 * "Bounded enums only, no unrestricted content" (16.6's own qualifier, matching Work Board's own
 * 14.5 discipline): every field here is either a bounded enum/array of enums or a short id the
 * reader picked from a real list (Agent) or typed as an exact-match filter (project id -- already
 * just as bounded as Work Board's own included `project` path). There is no free-text search
 * dimension in Mission Control's query model to exclude in the first place (unlike Work Board's
 * `text`), so unlike `captureWorkBoardSavedView`, nothing here needs to be deliberately dropped.
 */
export interface MissionControlSavedView {
  id: string;
  name: string;
  agentId: string;
  projectId: string;
  states: AgentRunState[];
  runner: "" | "local" | "ssh";
  sort: MissionControlSort;
}

const STORAGE_KEY = "vanehub.mission-control.saved-views.v1";
const CURRENT_VERSION = 1;

interface StoredPayload {
  version: number;
  views: MissionControlSavedView[];
}

function isOneOf<T extends string>(value: unknown, allowed: readonly T[]): value is T {
  return typeof value === "string" && (allowed as readonly string[]).includes(value);
}

function isValidStates(value: unknown): value is AgentRunState[] {
  return Array.isArray(value) && value.every((item) => (MISSION_CONTROL_REACHABLE_STATES as string[]).includes(item as string));
}

function isValidSavedView(value: unknown): value is MissionControlSavedView {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<MissionControlSavedView>;
  return typeof candidate.id === "string" && candidate.id.length > 0
    && typeof candidate.name === "string" && candidate.name.length > 0
    && typeof candidate.agentId === "string"
    && typeof candidate.projectId === "string"
    && isValidStates(candidate.states)
    && isOneOf(candidate.runner, runnerOptions)
    && isOneOf(candidate.sort, sortOptions);
}

/**
 * Mirrors `work-board-saved-views.ts`'s own versioning discipline, itself modeled on
 * `mission-control-view-state.ts`'s `.v1`-suffixed key: a whole-payload version mismatch discards
 * every saved view (a future format this build cannot understand at all); one malformed entry
 * within an otherwise-current payload is dropped on its own so it cannot take down the reader's
 * other saved views. Both fail closed to `[]` rather than throwing.
 */
export function readMissionControlSavedViews(): MissionControlSavedView[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Partial<StoredPayload>;
    if (parsed.version !== CURRENT_VERSION || !Array.isArray(parsed.views)) return [];
    return parsed.views.filter(isValidSavedView);
  } catch {
    return [];
  }
}

export function writeMissionControlSavedViews(views: MissionControlSavedView[]): void {
  if (typeof localStorage === "undefined") return;
  const payload: StoredPayload = { version: CURRENT_VERSION, views };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

/** Captures the current filter state under a trimmed, length-capped name -- see this file's own
 *  top-of-file comment for why nothing here needs to be excluded the way Work Board excludes its
 *  own free-text search. */
export function captureMissionControlSavedView(filter: MissionControlFilterState, name: string, id: string): MissionControlSavedView {
  return {
    agentId: filter.agentId,
    id,
    name: name.trim().slice(0, MISSION_CONTROL_SAVED_VIEW_NAME_MAX_LENGTH),
    projectId: filter.projectId,
    runner: filter.runner,
    sort: filter.sort,
    states: filter.states,
  };
}

export function applyMissionControlSavedView(view: MissionControlSavedView): MissionControlFilterState {
  return { agentId: view.agentId, projectId: view.projectId, runner: view.runner, sort: view.sort, states: view.states };
}
