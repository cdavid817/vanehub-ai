import { useCallback, useEffect, useMemo, useState } from "react";
import { useQueries, type QueryClient } from "@tanstack/react-query";
import { operationService } from "../../../services/runtime-operation-client";
import type { CliBulkItemResult } from "../../../types/cli-environment";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import { isOperationRunning } from "./cli-operation-status";

/**
 * Which operation belongs to which tool, and what it is doing.
 *
 * Kept out of the page so the page stays a layout: this is the only place that knows an operation
 * id maps to an agent, and the only place that decides when a finished operation invalidates the
 * snapshot list.
 *
 * Two maps rather than one flag, because "refreshing" and "mutating" disable different controls
 * and a single busy flag across all tools is exactly what this replaces.
 */
export function useCliOperationTracking(
  snapshots: readonly CliEnvironmentSnapshot[],
  queryClient: QueryClient,
  snapshotsQueryKey: readonly unknown[],
) {
  const [refreshing, setRefreshing] = useState<Record<string, string>>({});
  const [mutating, setMutating] = useState<Record<string, string>>({});

  const operationIds = useMemo(
    () => [...new Set([
      ...snapshots.flatMap((s) => (s.lastOperationId ? [s.lastOperationId] : [])),
      ...Object.values(refreshing),
      ...Object.values(mutating),
    ])],
    [snapshots, refreshing, mutating],
  );

  const queries = useQueries({
    queries: operationIds.map((operationId) => ({
      queryKey: ["operation", operationId],
      queryFn: () => operationService.getOperationStatus(operationId),
      refetchInterval: (query: { state: { data?: OperationTask } }) =>
        isOperationRunning(query.state.data) ? 1200 : false,
    })),
  });

  const operationsById = useMemo(() => {
    const entries: Array<[string, OperationTask]> = [];
    queries.forEach((query, index) => {
      if (query.data) entries.push([operationIds[index], query.data]);
    });
    return Object.fromEntries(entries);
  }, [operationIds, queries]);

  useEffect(() => {
    const finished = new Set(
      operationIds.filter((id) => operationsById[id] && !isOperationRunning(operationsById[id])),
    );
    if (finished.size === 0) return;
    setRefreshing((current) =>
      Object.fromEntries(Object.entries(current).filter(([, id]) => !finished.has(id))));
    setMutating((current) =>
      Object.fromEntries(Object.entries(current).filter(([, id]) => !finished.has(id))));
    // Only the CLI environment list. An unrelated query has no reason to refetch because a CLI
    // operation ended.
    void queryClient.invalidateQueries({ queryKey: snapshotsQueryKey });
  }, [operationIds, operationsById, queryClient, snapshotsQueryKey]);

  const operationsByAgentId = useMemo(() => {
    const byAgent: Record<string, OperationTask | undefined> = {};
    for (const snapshot of snapshots) {
      const id = mutating[snapshot.agentId] ?? refreshing[snapshot.agentId] ?? snapshot.lastOperationId;
      byAgent[snapshot.agentId] = id ? operationsById[id] : undefined;
    }
    return byAgent;
  }, [snapshots, mutating, refreshing, operationsById]);

  /** Polls one operation to its terminal state and returns its result payload. */
  const awaitResult = useCallback(async (operationId: string) => {
    for (let attempt = 0; attempt < 120; attempt += 1) {
      const task = await operationService.getOperationStatus(operationId);
      if (!isOperationRunning(task)) return task.result;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    return null;
  }, []);

  return {
    operationsByAgentId,
    refreshingAgentIds: new Set(Object.keys(refreshing)),
    mutatingAgentIds: new Set(Object.keys(mutating)),

    trackRefresh(operation: OperationTask, agentId: string | null, all: readonly CliEnvironmentSnapshot[]) {
      const targets = agentId ? [agentId] : all.map((snapshot) => snapshot.agentId);
      setRefreshing((current) => ({
        ...current,
        ...Object.fromEntries(targets.map((id) => [id, operation.id])),
      }));
    },

    trackMutation(operation: OperationTask, agentId: string | null) {
      if (!agentId) return;
      setMutating((current) => ({ ...current, [agentId]: operation.id }));
    },

    /** The plan id a preparation operation produced, once it finishes. */
    async awaitPlanId(operationId: string): Promise<string | null> {
      const result = await awaitResult(operationId);
      if (!result || typeof result !== "object" || Array.isArray(result)) return null;
      const planId = (result as { planId?: unknown }).planId;
      return typeof planId === "string" ? planId : null;
    },

    /** The per-item results a bulk execution produced, once it finishes. */
    async awaitBulkItems(operationId: string): Promise<readonly CliBulkItemResult[] | null> {
      const result = await awaitResult(operationId);
      if (!result || typeof result !== "object" || Array.isArray(result)) return null;
      const items = (result as { items?: unknown }).items;
      return Array.isArray(items) ? (items as CliBulkItemResult[]) : null;
    },

    cancel(agentId: string) {
      const operationId = mutating[agentId] ?? refreshing[agentId];
      if (!operationId) return;
      // Through the operation service, which owns the cancellation flag the backend polls.
      void operationService.cancelOperation(operationId);
    },
  };
}
