import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry, Session } from "../types/agent";
import type { AgentRunnerDescriptor, AgentRunnerSelection } from "../types/agent-runner";

const LOCAL: AgentRunnerSelection = { kind: "local" };

export function useRunnerSelection(
  session: Session | null,
  agents: AgentRegistryEntry[],
  service: Pick<typeof agentService, "listAgentRunners"> = agentService,
) {
  const [selection, setSelection] = useState<AgentRunnerSelection>(LOCAL);
  const query = useQuery({
    enabled: Boolean(session),
    queryKey: ["agent-runners", session?.id, session?.agentId],
    queryFn: () => session ? service.listAgentRunners(session.id, session.agentId) : Promise.resolve([]),
  });
  const apiLocalOnly = agents.find((agent) => agent.id === session?.agentId)?.launch.kind === "api";
  const descriptors = useMemo(() => (query.data ?? []).map((descriptor): AgentRunnerDescriptor =>
    apiLocalOnly && descriptor.selection.kind !== "local"
      ? { ...descriptor, available: false, unavailableReason: "runner_api_local_only" }
      : descriptor), [apiLocalOnly, query.data]);

  useEffect(() => { setSelection(LOCAL); }, [session?.agentId, session?.id]);
  useEffect(() => {
    if (query.isLoading) return;
    const selected = descriptors.find((descriptor) => sameSelection(descriptor.selection, selection));
    if (!selected?.available) setSelection(LOCAL);
  }, [descriptors, query.isLoading, selection]);

  return {
    descriptors,
    error: query.isError,
    loading: query.isLoading,
    refetch: query.refetch,
    selection,
    setSelection,
  };
}

export function sameSelection(left: AgentRunnerSelection, right: AgentRunnerSelection) {
  return left.kind === right.kind
    && (left.targetId ?? null) === (right.targetId ?? null)
    && (left.targetRevision ?? null) === (right.targetRevision ?? null);
}
