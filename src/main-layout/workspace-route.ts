export type WorkspaceDestination = "sessions" | "loops" | "work-board" | "goals" | "evaluations" | "mission-control" | "system-activity";

export const workspaceDestinations: WorkspaceDestination[] = ["sessions", "loops", "work-board", "goals", "evaluations", "mission-control", "system-activity"];

/** Reserved so it can never collide with a session id in `/workspace/sessions/<segment>`. */
const createSessionSegment = "new";
const storageKey = "vanehub.workspace.location.v1";

export interface WorkspaceLocation {
  destination: WorkspaceDestination;
  /** Only meaningful on the sessions destination. */
  sessionId: string | null;
  creatingSession: boolean;
}

export const defaultWorkspaceLocation: WorkspaceLocation = {
  destination: "sessions",
  sessionId: null,
  creatingSession: false,
};

function isDestination(value: string): value is WorkspaceDestination {
  return (workspaceDestinations as string[]).includes(value);
}

/**
 * Unknown destinations fall back to sessions rather than rendering an empty region, because the
 * workspace has no "not found" surface and a blank panel reads as a broken app.
 */
export function parseWorkspaceLocation(pathname: string): WorkspaceLocation {
  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== "workspace") return defaultWorkspaceLocation;
  const destination = segments[1];
  if (!destination || !isDestination(destination)) return defaultWorkspaceLocation;
  if (destination !== "sessions") return { destination, sessionId: null, creatingSession: false };
  const detail = segments[2] ? decodeURIComponent(segments[2]) : null;
  return {
    destination,
    sessionId: detail === createSessionSegment ? null : detail,
    creatingSession: detail === createSessionSegment,
  };
}

export function workspacePath(location: Partial<WorkspaceLocation>): string {
  const destination = location.destination ?? "sessions";
  if (destination !== "sessions") return `/workspace/${destination}`;
  if (location.creatingSession) return `/workspace/sessions/${createSessionSegment}`;
  return location.sessionId
    ? `/workspace/sessions/${encodeURIComponent(location.sessionId)}`
    : "/workspace/sessions";
}

/**
 * Remembered so a relaunch returns to where work stopped. The session id is intentionally
 * included: reopening on the sessions destination with nothing selected discards the one piece
 * of context the user most likely wanted back.
 */
export function rememberWorkspaceLocation(location: WorkspaceLocation): void {
  if (typeof localStorage === "undefined") return;
  // A half-finished creation is not a place worth returning to.
  const path = workspacePath({ ...location, creatingSession: false });
  localStorage.setItem(storageKey, path);
}

export function recallWorkspacePath(): string {
  if (typeof localStorage === "undefined") return workspacePath(defaultWorkspaceLocation);
  const stored = localStorage.getItem(storageKey);
  if (!stored?.startsWith("/workspace/")) return workspacePath(defaultWorkspaceLocation);
  return workspacePath(parseWorkspaceLocation(stored));
}
