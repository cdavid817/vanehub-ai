import type { KnownProject, KnownRemoteWorkspace, ProjectInspection, Session } from "../types/agent";
import type { SshConnection } from "../types/ssh-connection";
import { normalizeDisplayPath } from "../lib/session-path";
import type { SafeSessionSummary, WorkspaceSummary } from "./workspace-summary";

function toSafeSessionSummary(session: Session): SafeSessionSummary {
  return { id: session.id, title: session.title, lifecycleState: session.lifecycleState, updatedAt: session.updatedAt };
}

/** Most recently updated session among those already joined to one workspace, or none. */
function mostRecentSession(related: Session[]): SafeSessionSummary | undefined {
  if (!related.length) return undefined;
  const latest = related.reduce((newest, candidate) =>
    (Date.parse(candidate.updatedAt) > Date.parse(newest.updatedAt) ? candidate : newest));
  return toSafeSessionSummary(latest);
}

/**
 * Builds one local row. `inspection` is `null` when `inspectProject(project.path)` rejected —
 * the caller (the `useProjectWorkspaces` hook) is responsible for making that call and catching
 * the rejection; this function stays synchronous and side-effect-free so it can be unit-tested
 * with plain fixtures instead of mocked services.
 *
 * A real Tauri backend read confirms a missing path rejects `inspect_project` with
 * `WorkspaceApplicationError::Validation("Project unavailable")` (from `std::fs::canonicalize`
 * failing) rather than returning some "missing" flag inside a successful response — so "the call
 * rejected" is the only honest signal this increment has for 13.6's missing-path classification,
 * and `inspection === null` is how that signal is threaded through here.
 *
 * When inspection succeeded, `git.repository` reads the live `gitRoot` rather than the
 * project's own possibly-stale `isGit` (recorded whenever the project was last opened, which may
 * not reflect its current disk state). When inspection failed the path does not exist, so there
 * is nothing live to read; the last-known `isGit` is kept instead of dropping git context
 * entirely, since it is still a real historical fact, not a fabricated one.
 */
export function buildLocalWorkspaceSummary(
  project: KnownProject,
  sessions: Session[],
  inspection: ProjectInspection | null,
): WorkspaceSummary {
  const related = sessions.filter((session) => session.projectPath === project.path);
  return {
    availability: inspection ? "available" : "missing",
    displayName: project.displayName,
    displayPath: normalizeDisplayPath(project.path),
    git: { repository: inspection ? inspection.gitRoot !== null : project.isGit },
    kind: "local",
    lastOpenedAt: project.lastOpenedAt,
    recentSession: mostRecentSession(related),
    // No trust concept for local rows at all — see workspace-summary.ts's own field comment.
    workspaceId: project.path,
  };
}

/** Case-insensitive host, exact port/user — matches how SSH connection identity is compared elsewhere in this codebase. */
function sshIdentityKey(host: string, port: number, user: string): string {
  return `${host.trim().toLowerCase()}:${port}:${user.trim()}`;
}

/**
 * The only plausible join between a remembered remote path and a saved connection profile: no
 * shared foreign key exists between `KnownRemoteWorkspace` and `SshConnection` (confirmed by
 * reading both types), so host+port+user identity is the closest thing to one. `remote.port`
 * defaults to 22 the same way `normalizeRemoteWorkspace` already does when constructing a
 * `RemoteWorkspace`, so an omitted port still matches a connection saved with the explicit
 * default. This can legitimately find no match — a remembered path with no saved profile for its
 * host/port/user — which every caller below must handle rather than assume away.
 */
function matchConnection(remote: KnownRemoteWorkspace, connections: SshConnection[]): SshConnection | undefined {
  const key = sshIdentityKey(remote.host, remote.port ?? 22, remote.user ?? "");
  return connections.find((connection) => sshIdentityKey(connection.host, connection.port, connection.user) === key);
}

function remoteDisplayPath(remote: KnownRemoteWorkspace): string {
  const authority = remote.user ? `${remote.user}@${remote.host}` : remote.host;
  const port = remote.port && remote.port !== 22 ? `:${remote.port}` : "";
  return `${authority}${port}:${normalizeDisplayPath(remote.path)}`;
}

/**
 * Builds one SSH row. When `matchConnection` finds a profile, sessions are joined by
 * `remoteSshConnectionId` (the stable, service-assigned link); when it does not, sessions are
 * joined by the session's own embedded `remoteWorkspace.uri` as a fallback — both shapes are
 * named explicitly in this increment's brief rather than invented here.
 *
 * Trust and availability both collapse two distinct "not confirmed" situations onto the same
 * output value, documented individually below, because the target enums have no room for a
 * third state — not because the situations are actually the same.
 */
export function buildRemoteWorkspaceSummary(
  remote: KnownRemoteWorkspace,
  connections: SshConnection[],
  sessions: Session[],
): WorkspaceSummary {
  const connection = matchConnection(remote, connections);
  const related = connection
    ? sessions.filter((session) => session.remoteSshConnectionId === connection.id)
    : sessions.filter((session) => session.remoteWorkspace?.uri === remote.uri);
  return {
    // "unknown" covers both "no saved connection profile at all" and "a profile exists but its
    // host key has never been confirmed" (`hostTrust === null`) — neither one is evidence of
    // `"trusted"`, and nothing anywhere records `"untrusted"`/`"revoked"` (see workspace-summary.ts).
    availability: connectionAvailability(connection),
    // Same `connection` this function already matched to derive trust/availability/recentSession
    // above -- not a second lookup, just also kept for task 13.8's Reconnect action (see
    // `WorkspaceSummary.connectionId`'s own doc comment for why it cannot be safely re-derived
    // anywhere else).
    connectionId: connection?.id,
    displayName: remote.displayName,
    displayPath: remoteDisplayPath(remote),
    kind: "ssh",
    lastOpenedAt: remote.lastOpenedAt,
    recentSession: mostRecentSession(related),
    trust: connection?.hostTrust ? "trusted" : "unknown",
    workspaceId: remote.uri,
  };
}

/**
 * `testStatus` has no "unknown" option to mirror `"not-tested"` onto — and "no connection profile
 * matched at all" carries even less confirmation than "a profile exists but was never tested".
 * Both collapse to `"disconnected"` (never confirmed reachable) rather than `"available"`, which
 * would claim a confirmation that never happened. `"missing"` is not used here at all — see
 * `WorkspaceAvailability`'s own comment for why that value is local-only.
 */
function connectionAvailability(connection: SshConnection | undefined): WorkspaceSummary["availability"] {
  return connection?.testStatus === "succeeded" ? "available" : "disconnected";
}

export interface WorkspaceAggregationInput {
  projects: KnownProject[];
  /** Keyed by `KnownProject.path`; `null` marks an `inspectProject` rejection (missing path). */
  inspections: ReadonlyMap<string, ProjectInspection | null>;
  remoteWorkspaces: KnownRemoteWorkspace[];
  connections: SshConnection[];
  sessions: Session[];
}

/** Combines local and remote rows into the one list the Projects page renders. */
export function buildWorkspaceSummaries(input: WorkspaceAggregationInput): WorkspaceSummary[] {
  const local = input.projects.map((project) =>
    buildLocalWorkspaceSummary(project, input.sessions, input.inspections.get(project.path) ?? null));
  const remote = input.remoteWorkspaces.map((remote) =>
    buildRemoteWorkspaceSummary(remote, input.connections, input.sessions));
  return [...local, ...remote];
}
