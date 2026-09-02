import { useCallback, useEffect, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import type { ProjectInspection } from "../types/agent";
import { buildWorkspaceSummaries } from "./workspace-aggregation";
import type { WorkspaceSummary } from "./workspace-summary";

/**
 * One `inspectProject` call per known local project. This is real but N+1-shaped, a deliberate
 * trade-off rather than an oversight: 13.6 requires missing local paths to be classified
 * correctly, and nothing short of a live filesystem check (which `inspectProject` is the only
 * existing service method for) can tell a path that still exists from one that does not — see
 * `workspace-aggregation.ts` for the confirmed rejection shape this depends on. Local project
 * history realistically stays in the tens, the same "not meant to be a paged search" scale
 * `session-execution-context.ts` documents for its own bounded scan, so this is not expected to
 * grow into a real performance problem before 13.12's list-then-detail composition revisits it.
 * A rejection (missing path, or any other failure) is recorded as `null`, never guessed at.
 */
async function inspectAll(paths: string[]): Promise<Map<string, ProjectInspection | null>> {
  const entries = await Promise.all(paths.map(async (path): Promise<readonly [string, ProjectInspection | null]> => {
    try {
      return [path, await agentService.inspectProject(path)];
    } catch {
      return [path, null];
    }
  }));
  return new Map(entries);
}

/**
 * Fetches from the existing project-history, SSH-connection, and session services and joins them
 * into `WorkspaceSummary[]` via the pure functions in `workspace-aggregation.ts`. No new Tauri
 * command or service interface is introduced — this mirrors Mission Control's own
 * `session-execution-context.ts` precedent of a small client-side join over already-existing
 * service boundaries (design.md Decision 18: Projects is a read-only aggregation, not a new
 * cross-domain truth table).
 *
 * `data` starts `undefined` (not an empty array) so callers can distinguish "still loading" from
 * "loaded, and there is genuinely nothing" the same way `AsyncViewState<T>` already expects.
 */
export function useProjectWorkspaces() {
  const [data, setData] = useState<WorkspaceSummary[] | undefined>(undefined);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [projects, remoteWorkspaces, connections, sessions] = await Promise.all([
        agentService.listKnownProjects(),
        agentService.listKnownRemoteWorkspaces(),
        sshConnectionService.listConnections(),
        agentService.listSessions(),
      ]);
      const inspections = await inspectAll(projects.map((project) => project.path));
      setData(buildWorkspaceSummaries({ connections, inspections, projects, remoteWorkspaces, sessions }));
      setError(null);
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  return { data, error, loading, reload: load };
}
