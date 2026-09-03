import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { SessionLogEntry } from "../types/session-workspace";
import type { AsyncViewState } from "../ui/async/async-view-state";
import { resolveSessionExecutionContext } from "./session-execution-context";

// A run detail panel, not a full log viewer — mirrors use-mission-control-usage.ts's own
// INVOCATION_LIST_LIMIT reasoning: enough to show real recent activity without paging. The full,
// paginated Session Log tab (session-workspace/use-session-logs.ts) already owns the "read
// everything" case; this facet is a bounded summary, not a second implementation of that.
export const LOG_LIST_LIMIT = 20;

export interface MissionControlLogsData {
  entries: SessionLogEntry[];
  truncated: boolean;
}

/**
 * The Logs facet's own fetch (16.12): a single bounded, run-correlated page from the unified
 * session-log index (`agentService.listSessionLogs`), not an unbounded dump and not the whole
 * session's logs.
 *
 * `runId` in `SessionLogQuery` correlates against the *execution*-context run id every real
 * production log call site tags a record with (confirmed by reading every non-test `run_id:
 * Some(...)` construction under `src-tauri/src/contexts/agent_runtime/`: all of them read
 * `execution_context.run_id` / `context.run_id`, never `AgentRun.id`) — the same `ExecutionRunId`
 * `resolveSessionExecutionContext` already resolves for Timeline/Tools/Files/Usage. Reusing that
 * resolver here, rather than querying by `sessionId` alone, is what keeps this "logs for this Run,"
 * not "every log line for this Run's whole session" (design.md's own "SHALL NOT load unrelated Run
 * logs" for the Logs section) — a session can legitimately host more than one Run.
 *
 * No resolvable session/execution run is reported the same honest way `useExecutionTimeline` reports
 * it: `error: { kind: "unavailable" }`, not a fetch failure.
 */
export function useMissionControlLogs(
  run: MissionControlRunSummary,
  unavailableMessage: string,
  errorMessage: string,
): AsyncViewState<MissionControlLogsData> & { reload: () => void } {
  const [state, setState] = useState<AsyncViewState<MissionControlLogsData>>({
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
      const page = await agentService.listSessionLogs({
        sessionId, runId: resolved.runId, levels: [], search: "", limit: LOG_LIST_LIMIT,
      });
      if (attempt !== generation.current) return;
      setState({ data: { entries: page.items, truncated: page.truncated }, initialLoading: false, refreshing: false, stale: false });
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
