import { agentService } from "../services/runtime-agent-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import type { KnownProject } from "../types/agent";
import type { SshConnection } from "../types/ssh-connection";
import type { WorkbenchSearchProvider, WorkbenchSearchRequest, WorkbenchSearchResult } from "./command-center-types";

/**
 * 6.4. Neither `listKnownProjects()` nor `sshConnectionService.listConnections()` takes a query —
 * both return everything unconditionally, so filtering happens here, client-side.
 *
 * An empty query matches nothing, deliberately: the simpler of the two reasonable defaults (the
 * other being "show the N most recent"), and it means this provider never has to guess what a
 * caller wants to see before the reader has typed anything.
 */
function matchesQuery(query: string, ...fields: string[]): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return false;
  return fields.some((field) => field.toLowerCase().includes(needle));
}

function fromProject(project: KnownProject): WorkbenchSearchResult {
  return {
    key: project.path,
    kind: "project",
    title: project.displayName,
    subtitle: project.path,
    route: { destination: "projects", projectId: project.path },
    updatedAt: project.lastOpenedAt,
  };
}

// No dedicated "SSH" scope exists in WorkbenchSearchScope (design.md Decision 4 names exactly
// session/project/run/goal/work-item/evaluation) — an SSH connection is a kind of addressable
// project/workspace, so it reuses "project" rather than growing the scope union for one source.
function fromConnection(connection: SshConnection): WorkbenchSearchResult {
  return {
    key: connection.id,
    kind: "project",
    title: connection.name,
    // Never `lastError` — design.md Decision 4 forbids raw errors in search results, and this
    // field is exactly that: an uncontrolled string from a failed connection attempt.
    subtitle: `${connection.user}@${connection.host}`,
    route: { destination: "projects", projectId: connection.id },
  };
}

// `route` above points at ProjectsDestination (main-layout/projects-destination.tsx), which is
// still an empty-state placeholder today (task group 13, a later milestone) — a result here is
// addressable but not yet rich content once opened. Not this provider's gap to close.
export const projectSearchProvider: WorkbenchSearchProvider = {
  id: "projects",
  supports: (scope) => scope === "project",
  // request.signal is intentionally unused: neither source call is abortable, and a shared
  // orchestrator discards stale results centrally rather than each provider doing it separately.
  async search(request: WorkbenchSearchRequest) {
    const [projects, connections] = await Promise.all([
      agentService.listKnownProjects(),
      sshConnectionService.listConnections(),
    ]);
    const matchedProjects = projects
      .filter((project) => matchesQuery(request.query, project.displayName, project.path))
      .map(fromProject);
    const matchedConnections = connections
      .filter((connection) => matchesQuery(request.query, connection.name, connection.host))
      .map(fromConnection);
    // Combined limit, not per-source: the two sources are one merged result list to the caller,
    // and a caller asking for 10 results should get at most 10, not up to 20.
    const items = [...matchedProjects, ...matchedConnections].slice(0, request.limit);
    return { items, nextCursor: null };
  },
};
