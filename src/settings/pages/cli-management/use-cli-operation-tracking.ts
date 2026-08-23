import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQueries, type QueryClient } from "@tanstack/react-query";
import { operationService } from "../../../services/runtime-operation-client";
import type { CliBulkItemResult } from "../../../types/cli-environment";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import { isOperationRunning } from "./cli-operation-status";

/** Returns the same object when nothing was dropped, so setting state cannot force a re-render. */
function withoutOperations(
  current: Record<string, string>,
  finished: ReadonlySet<string>,
): Record<string, string> {
  const kept = Object.entries(current).filter(([, id]) => !finished.has(id));
  return kept.length === Object.keys(current).length ? current : Object.fromEntries(kept);
}

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
  /**
   * The most recent operation per tool, kept after it finishes.
   *
   * Separate from the two busy maps on purpose. Those are pruned the moment an operation settles,
   * which is right for disabling controls and wrong for showing a result: pruning the displayed
   * operation too made every terminal outcome vanish in the same render it arrived in, so a user
   * never saw whether the change they authorized worked.
   */
  const [shown, setShown] = useState<Record<string, string>>({});

  const operationIds = useMemo(
    () => [...new Set([
      ...snapshots.flatMap((s) => (s.lastOperationId ? [s.lastOperationId] : [])),
      ...Object.values(refreshing),
      ...Object.values(mutating),
      ...Object.values(shown),
    ])],
    [snapshots, refreshing, mutating, shown],
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

  // Which operations have already been reacted to, and which were seen mid-flight. `useQueries`
  // hands back a fresh array every render, so the effect below re-runs constantly; without these
  // it re-set both maps to brand-new empty objects each time, and re-setting state from an effect
  // whose own dependency changes on every render is a render loop that ends in a dead worker.
  const settled = useRef(new Set<string>());
  const seenRunning = useRef(new Set<string>());

  useEffect(() => {
    const started = new Set([...Object.values(refreshing), ...Object.values(mutating)]);
    const finished = new Set<string>();
    for (const id of operationIds) {
      const task = operationsById[id];
      if (!task) continue;
      if (isOperationRunning(task)) {
        seenRunning.current.add(id);
        continue;
      }
      if (settled.current.has(id)) continue;
      settled.current.add(id);
      // An operation already finished when this page first looked at it changed nothing while the
      // page was watching, so there is nothing to invalidate on its account.
      if (seenRunning.current.has(id) || started.has(id)) finished.add(id);
    }
    if (finished.size === 0) return;
    setRefreshing((current) => withoutOperations(current, finished));
    setMutating((current) => withoutOperations(current, finished));
    // Only the CLI environment list. An unrelated query has no reason to refetch because a CLI
    // operation ended.
    void queryClient.invalidateQueries({ queryKey: snapshotsQueryKey });
  }, [operationIds, operationsById, refreshing, mutating, queryClient, snapshotsQueryKey]);

  /**
   * The operation a tool shows and its controls act on.
   *
   * `lastOperationId` is in the chain because an operation started before this page mounted -- or
   * by another window -- is still the one running against that tool. Leaving it out gave the card
   * an operation to display and no operation to cancel.
   */
  const operationIdByAgentId = useMemo(() => {
    const byAgent: Record<string, string | undefined> = {};
    for (const snapshot of snapshots) {
      byAgent[snapshot.agentId] = mutating[snapshot.agentId]
        ?? refreshing[snapshot.agentId]
        ?? shown[snapshot.agentId]
        ?? snapshot.lastOperationId
        ?? undefined;
    }
    return byAgent;
  }, [snapshots, mutating, refreshing, shown]);

  const operationsByAgentId = useMemo(() => {
    const byAgent: Record<string, OperationTask | undefined> = {};
    for (const [agentId, id] of Object.entries(operationIdByAgentId)) {
      byAgent[agentId] = id ? operationsById[id] : undefined;
    }
    return byAgent;
  }, [operationIdByAgentId, operationsById]);

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
      const assigned = Object.fromEntries(targets.map((id) => [id, operation.id]));
      setRefreshing((current) => ({ ...current, ...assigned }));
      setShown((current) => ({ ...current, ...assigned }));
    },

    trackMutation(operation: OperationTask, agentId: string | null) {
      if (!agentId) return;
      setMutating((current) => ({ ...current, [agentId]: operation.id }));
      setShown((current) => ({ ...current, [agentId]: operation.id }));
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
      const operationId = operationIdByAgentId[agentId];
      // A finished operation stays on the card so its result can be read; cancelling one would ask
      // the backend to stop work that already stopped.
      if (!operationId || !isOperationRunning(operationsById[operationId])) return;
      // Through the operation service, which owns the cancellation flag the backend polls.
      void operationService.cancelOperation(operationId);
    },
  };
}
