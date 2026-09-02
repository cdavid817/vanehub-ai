import type { WorkspaceSummary } from "./workspace-summary";

/**
 * 13.4 (partial): only the three views that need no new product decision are built here.
 * "favorite" needs a field that does not exist anywhere in `KnownProject`/`RemoteWorkspace`/
 * `SshConnection` — inventing local-only favorite state would be a real, silent product decision
 * (does it sync across devices? does it belong in Settings' own Saved View pattern instead?), so
 * it is left out entirely rather than built hollow. "needs-attention" would need the same blocked
 * `activeRuns`/trust-change signals `WorkspaceSummary` itself does not carry in this increment —
 * see `workspace-summary.ts`.
 */
export type WorkspaceView = "recent" | "all" | "unavailable";

export const workspaceViews: WorkspaceView[] = ["recent", "all", "unavailable"];

/**
 * Small and arbitrary by necessity: design.md does not define "recent" beyond the view's name,
 * and no other destination in this codebase has an established page size for this kind of list
 * yet (13.12's compact list-then-detail composition, which would likely replace this with real
 * pagination, is explicitly out of scope for this increment).
 */
const RECENT_VIEW_LIMIT = 8;

function recencyKey(workspace: WorkspaceSummary): number {
  return workspace.lastOpenedAt ? Date.parse(workspace.lastOpenedAt) : 0;
}

function sortByRecency(workspaces: WorkspaceSummary[]): WorkspaceSummary[] {
  return [...workspaces].sort((left, right) => recencyKey(right) - recencyKey(left));
}

/**
 * Client-side only, over the already-aggregated list — no new service call per view. "recent" is
 * a bounded slice of a non-empty list, so it can only be empty when `workspaces` itself is; "all"
 * shows everything unfiltered. Only "unavailable" can be empty while `workspaces` is not (every
 * known workspace is reachable), which is why the Projects page only needs one view-specific
 * empty-state message.
 */
export function selectWorkspaceView(workspaces: WorkspaceSummary[], view: WorkspaceView): WorkspaceSummary[] {
  const sorted = sortByRecency(workspaces);
  if (view === "unavailable") return sorted.filter((workspace) => workspace.availability !== "available");
  if (view === "recent") return sorted.slice(0, RECENT_VIEW_LIMIT);
  return sorted;
}
