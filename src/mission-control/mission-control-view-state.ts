import type { MissionControlSort } from "../types/mission-control";

/**
 * 4.8: "preserve filter, sort, ... state when navigating to an authoritative evidence surface and
 * back." `sessionStorage`, not `localStorage` — this is the reader's current working filter, not a
 * durable preference; a fresh app launch days later should not silently re-apply a stale runner
 * filter the reader has long forgotten setting.
 */
export interface MissionControlViewState {
  status: string;
  agentId: string;
  projectId: string;
  runner: "" | "local" | "ssh";
  sort: MissionControlSort;
}

const VIEW_STATE_KEY = "vanehub.mission-control.view.v1";
const SCROLL_KEY = "vanehub.mission-control.scroll.v1";

export function readMissionControlViewState(): MissionControlViewState | null {
  if (typeof sessionStorage === "undefined") return null;
  try {
    const raw = sessionStorage.getItem(VIEW_STATE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<MissionControlViewState>;
    if (typeof parsed.status !== "string" || typeof parsed.agentId !== "string" || typeof parsed.projectId !== "string") return null;
    if (parsed.runner !== "" && parsed.runner !== "local" && parsed.runner !== "ssh") return null;
    if (parsed.sort !== "newest" && parsed.sort !== "oldest" && parsed.sort !== "attention") return null;
    return { status: parsed.status, agentId: parsed.agentId, projectId: parsed.projectId, runner: parsed.runner, sort: parsed.sort };
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
