import { useQuery } from "@tanstack/react-query";
import type { AgentService } from "../../../services/agent-service";
import type { AgentPersonalizationCapability } from "../../../types/personalization";

export interface WorkspaceOption {
  /** The stable key a scope is addressed by, resolved natively. */
  workspaceKey: string;
  /** What the user reads. Never the identity: two remote paths can look alike and must not merge. */
  displayName: string;
  kind: "local" | "remote";
}

/**
 * The Agents and workspaces a scope can be selected for.
 *
 * Both lists are asked for rather than declared. An Agent registered after this shipped has to
 * appear without an edit here, and a workspace key can only come from the native resolver -- a
 * frontend that derived one would be the second subsystem deciding what "the same workspace" means,
 * which is how one workspace's policy starts applying to another.
 */
export function useScopeOptions(service: AgentService) {
  const agentsQuery = useQuery({
    queryKey: ["personalization", "agent-capabilities"] as const,
    queryFn: () => service.listPersonalizationAgentCapabilities(),
  });

  const workspacesQuery = useQuery({
    queryKey: ["personalization", "workspace-options"] as const,
    queryFn: () => loadWorkspaceOptions(service),
  });

  return {
    agents: (agentsQuery.data ?? []) as AgentPersonalizationCapability[],
    workspaces: workspacesQuery.data ?? [],
    isPending: agentsQuery.isPending || workspacesQuery.isPending,
    error: agentsQuery.error ?? workspacesQuery.error,
  };
}

async function loadWorkspaceOptions(service: AgentService): Promise<WorkspaceOption[]> {
  const [projects, remotes] = await Promise.all([
    service.listKnownProjects(),
    service.listKnownRemoteWorkspaces(),
  ]);

  const resolved = await Promise.all([
    ...projects.map(async (project) => {
      const scope = await service.resolvePersonalizationWorkspace({ projectPath: project.path });
      return scope ? { ...scope, displayName: project.displayName } : null;
    }),
    ...remotes.map(async (remote) => {
      // Sent as parts, never as `remote.uri`: a URI can carry `user:password@host`, and there is
      // nowhere in these fields to put one.
      const scope = await service.resolvePersonalizationWorkspace({
        remote: {
          host: remote.host,
          port: remote.port ?? undefined,
          user: remote.user ?? undefined,
          path: remote.path,
        },
      });
      return scope ? { ...scope, displayName: remote.displayName } : null;
    }),
  ]);

  // Deduplicated by key, because a worktree and its project can resolve to the same workspace and
  // two entries for one scope would let a user edit the same layer from two rows.
  const byKey = new Map<string, WorkspaceOption>();
  for (const option of resolved) {
    if (option && !byKey.has(option.workspaceKey)) byKey.set(option.workspaceKey, option);
  }
  return [...byKey.values()];
}
