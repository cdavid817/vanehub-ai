import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { ScheduledTaskRun } from "../types/agent";

/**
 * 19.11: fetches `listScheduledTaskRuns` for whichever task id is currently selected, returning
 * `AsyncViewState<ScheduledTaskRun[]>` so the detail view can render it through the same
 * `AsyncBoundary` every other list-shaped fetch in this codebase already uses, instead of a
 * bespoke loading/error shape.
 *
 * `generation` guards against a fast reselect (task A -> task B -> task A) letting a slow
 * in-flight response for a task the reader has since navigated away from land after a newer one
 * already resolved -- the same ref-guarded-by-attempt-number idiom `use-session-logs.ts`'s own
 * `loadFirstPage` established for an identical race (there: session id; here: task id).
 */
export function useScheduledTaskHistory(taskId: string | null): AsyncViewState<ScheduledTaskRun[]> & { reload: () => void } {
  const [state, setState] = useState<AsyncViewState<ScheduledTaskRun[]>>({
    data: undefined, initialLoading: false, refreshing: false, stale: false,
  });
  const generation = useRef(0);

  const load = useCallback(() => {
    if (!taskId) {
      setState({ data: undefined, initialLoading: false, refreshing: false, stale: false });
      return;
    }
    const attempt = ++generation.current;
    setState((current) => ({
      data: current.data, initialLoading: current.data === undefined, refreshing: current.data !== undefined, stale: false,
    }));
    void agentService.listScheduledTaskRuns(taskId).then((runs) => {
      if (attempt !== generation.current) return;
      setState({ data: runs, initialLoading: false, refreshing: false, stale: false });
    }).catch((reason: unknown) => {
      if (attempt !== generation.current) return;
      setState((current) => ({
        data: current.data,
        initialLoading: false,
        refreshing: false,
        stale: false,
        error: { kind: "error", message: reason instanceof Error ? reason.message : String(reason), retryable: true },
      }));
    });
  }, [taskId]);

  useEffect(() => { load(); }, [load]);

  return { ...state, reload: load };
}
