export type WorkspaceDestination = "sessions" | "inbox" | "automations";
export type InboxView = "attention" | "running" | "recent" | "board";
export type AutomationsView = "loops" | "scheduled" | "goals";
export type WorkspaceView = InboxView | AutomationsView;

export const workspaceDestinations: WorkspaceDestination[] = ["sessions", "inbox", "automations"];
export const inboxViews: InboxView[] = ["attention", "running", "recent", "board"];
export const automationsViews: AutomationsView[] = ["loops", "scheduled", "goals"];
export const defaultInboxView: InboxView = "attention";
export const defaultAutomationsView: AutomationsView = "loops";

/** Reserved so it can never collide with a session id in `/workspace/sessions/<segment>`. */
const createSessionSegment = "new";
const storageKey = "vanehub.workspace.location.v1";

export interface WorkspaceLocation {
  destination: WorkspaceDestination;
  /** Sub-view of inbox or automations; always null on the sessions destination. */
  view: WorkspaceView | null;
  /** Only meaningful on the sessions destination. */
  sessionId: string | null;
  creatingSession: boolean;
}

export type WorkspaceRouteResolution =
  /** `legacy` marks a retired path: the route layer rewrites the URL to the host it resolved to. */
  | { kind: "workspace"; location: WorkspaceLocation; legacy: boolean }
  /** A retired path whose host is now a settings page; the location is what renders behind it. */
  | { kind: "settings"; pageId: "evaluation"; location: WorkspaceLocation };

export const defaultWorkspaceLocation: WorkspaceLocation = {
  destination: "sessions",
  view: null,
  sessionId: null,
  creatingSession: false,
};

/**
 * Retired destinations keep resolving so bookmarks, remembered locations, and older slash
 * commands land on the surface that now hosts them rather than on a blank panel.
 */
const legacyDestinationRedirects: Record<string, { destination: WorkspaceDestination; view: WorkspaceView }> = {
  "mission-control": { destination: "inbox", view: "attention" },
  "system-activity": { destination: "inbox", view: "attention" },
  "work-board": { destination: "inbox", view: "board" },
  loops: { destination: "automations", view: "loops" },
  goals: { destination: "automations", view: "goals" },
};
const legacySettingsRedirects: Record<string, "evaluation"> = { evaluations: "evaluation" };

function isDestination(value: string): value is WorkspaceDestination {
  return (workspaceDestinations as string[]).includes(value);
}

export function defaultWorkspaceView(destination: WorkspaceDestination): WorkspaceView | null {
  if (destination === "inbox") return defaultInboxView;
  if (destination === "automations") return defaultAutomationsView;
  return null;
}

/** An unknown or foreign view falls back to the destination's default rather than to nothing. */
export function normalizeWorkspaceView(destination: WorkspaceDestination, view: string | null | undefined): WorkspaceView | null {
  if (destination === "inbox") return view && (inboxViews as string[]).includes(view) ? view as InboxView : defaultInboxView;
  if (destination === "automations") {
    return view && (automationsViews as string[]).includes(view) ? view as AutomationsView : defaultAutomationsView;
  }
  return null;
}

/**
 * Unknown destinations fall back to sessions rather than rendering an empty region, because the
 * workspace has no "not found" surface and a blank panel reads as a broken app.
 */
export function resolveWorkspaceRoute(pathname: string): WorkspaceRouteResolution {
  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== "workspace") return { kind: "workspace", location: defaultWorkspaceLocation, legacy: false };
  const destination = segments[1];
  if (!destination) return { kind: "workspace", location: defaultWorkspaceLocation, legacy: false };
  const settingsPage = legacySettingsRedirects[destination];
  if (settingsPage) return { kind: "settings", pageId: settingsPage, location: defaultWorkspaceLocation };
  const redirect = legacyDestinationRedirects[destination];
  if (redirect) return { kind: "workspace", location: { ...redirect, sessionId: null, creatingSession: false }, legacy: true };
  if (!isDestination(destination)) return { kind: "workspace", location: defaultWorkspaceLocation, legacy: false };
  if (destination !== "sessions") {
    const view = normalizeWorkspaceView(destination, segments[2] ? decodeURIComponent(segments[2]) : null);
    return { kind: "workspace", location: { destination, view, sessionId: null, creatingSession: false }, legacy: false };
  }
  const detail = segments[2] ? decodeURIComponent(segments[2]) : null;
  return {
    kind: "workspace",
    legacy: false,
    location: {
      destination,
      view: null,
      sessionId: detail === createSessionSegment ? null : detail,
      creatingSession: detail === createSessionSegment,
    },
  };
}

export function parseWorkspaceLocation(pathname: string): WorkspaceLocation {
  return resolveWorkspaceRoute(pathname).location;
}

export function workspacePath(location: Partial<WorkspaceLocation>): string {
  const destination = location.destination ?? "sessions";
  if (destination !== "sessions") return `/workspace/${destination}/${normalizeWorkspaceView(destination, location.view)}`;
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

/**
 * A remembered retired path is returned verbatim rather than normalized, so the workspace route
 * applies the same redirect it applies to a typed URL — including the one that opens settings.
 */
export function recallWorkspacePath(): string {
  if (typeof localStorage === "undefined") return workspacePath(defaultWorkspaceLocation);
  const stored = localStorage.getItem(storageKey);
  if (!stored?.startsWith("/workspace/")) return workspacePath(defaultWorkspaceLocation);
  const resolution = resolveWorkspaceRoute(stored);
  return resolution.kind === "settings" ? stored : workspacePath(resolution.location);
}
