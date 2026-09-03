import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { ModelInvocation, TokenUsageSummary } from "../types/token-usage";
import type { AsyncViewState } from "../ui/async/async-view-state";
import { resolveSessionExecutionContext } from "./session-execution-context";

// A run detail panel, not a dashboard — enough to show real recent activity without paging.
const INVOCATION_LIST_LIMIT = 20;

export interface MissionControlUsageData {
  summary: TokenUsageSummary;
  invocations: ModelInvocation[];
}

/**
 * The Usage facet's own fetch: joins this Mission Control run to its execution-observability run
 * via `resolveSessionExecutionContext` (the same join `useExecutionTimeline` uses), then loads real
 * token-usage content for that session.
 *
 * The backend's own `MissionControlFacetAvailability` for "usage" gates whether the facet is even
 * mounted (see `mission-control-facets.tsx`) — this hook does not re-decide availability, it only
 * decides what to fetch once mounted, including an honest `"unavailable"` state when the resolver
 * itself cannot find anything (e.g. the concurrent-runs edge case documented on the resolver).
 *
 * `unavailableMessage`/`errorMessage` are already-translated strings from the caller, mirroring
 * `useExecutionTimeline`'s own reasoning for taking plain strings rather than an options object or
 * i18n keys resolved in here.
 */
export function useMissionControlUsage(
  run: MissionControlRunSummary,
  unavailableMessage: string,
  errorMessage: string,
): AsyncViewState<MissionControlUsageData> & { reload: () => void } {
  const [state, setState] = useState<AsyncViewState<MissionControlUsageData>>({
    data: undefined, initialLoading: true, refreshing: false, stale: false,
  });
  const generation = useRef(0);

  const load = useCallback(() => {
    const attempt = ++generation.current;
    setState((current) => ({
      data: current.data, initialLoading: current.data === undefined, refreshing: current.data !== undefined, stale: false,
    }));
    void (async () => {
      const resolved = await resolveSessionExecutionContext(run, executionObservabilityService);
      const sessionId = resolved?.sessionId;
      if (!sessionId) {
        if (attempt !== generation.current) return;
        setState({
          data: undefined, initialLoading: false, refreshing: false, stale: false,
          error: { kind: "unavailable", message: unavailableMessage, retryable: false },
        });
        return;
      }
      const [summary, details] = await Promise.all([
        agentService.getTokenUsageSummary({ sessionId }),
        agentService.getTokenUsageDetails({ sessionId, limit: INVOCATION_LIST_LIMIT }),
      ]);
      if (attempt !== generation.current) return;
      setState({ data: { summary, invocations: details.invocations }, initialLoading: false, refreshing: false, stale: false });
    })().catch(() => {
      if (attempt !== generation.current) return;
      setState((current) => ({
        data: current.data, initialLoading: false, refreshing: false, stale: false,
        error: { kind: "error", message: errorMessage, retryable: true },
      }));
    });
  }, [run, unavailableMessage, errorMessage]);

  useEffect(() => { load(); }, [load]);

  return { ...state, reload: load };
}
