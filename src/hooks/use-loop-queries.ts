import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { LoopRun } from "../types/loop";
import { applyLoopRunUpdate, loopQueryKeys, preserveLoopRuns } from "./loop-query";

export function useLoopDefinitionsQuery() {
  return useQuery({
    queryKey: loopQueryKeys.definitions,
    queryFn: () => agentService.listLoopDefinitions(),
  });
}

export function useLoopRunsQuery(definitionId?: string) {
  return useQuery({
    queryKey: loopQueryKeys.runs(definitionId),
    queryFn: () => agentService.listLoopRuns(definitionId),
    placeholderData: preserveLoopRuns,
  });
}

export function useLoopRunQuery(runId: string | null) {
  const queryClient = useQueryClient();
  const query = useQuery({
    enabled: Boolean(runId),
    queryKey: loopQueryKeys.run(runId ?? ""),
    queryFn: () => agentService.getLoopRun(runId ?? ""),
  });

  useEffect(() => {
    if (!runId) return;
    let active = true;
    let cleanup: (() => void) | undefined;
    void agentService.subscribeLoopEvents(runId, ({ run }) => {
      queryClient.setQueryData(loopQueryKeys.run(run.id), run);
      queryClient.setQueriesData<LoopRun[]>(
        { queryKey: ["loops", "runs"] },
        (current) => applyLoopRunUpdate(current, run),
      );
    }).then((unsubscribe) => {
      if (active) cleanup = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      cleanup?.();
    };
  }, [queryClient, runId]);

  return query;
}
