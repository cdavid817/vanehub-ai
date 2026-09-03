import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { ScheduledTaskRun } from "../types/agent";

export interface UseScheduledTaskHistoryResult extends AsyncViewState<ScheduledTaskRun[]> {
  reload: () => void;
  /** 19.11: whether the service reports a further page beyond what `data` currently holds. */
  hasMore: boolean;
  /** True only while a `loadMore()` request is in flight, so the caller can disable its own
   *  "load more" control instead of letting a reader queue up duplicate page requests. */
  loadingMore: boolean;
  /** No-op while there is no further page, no selected task, or a request is already in flight.
   *  Appends the next page to the already-loaded rows -- mirrors `useEvaluationQuery`'s own
   *  `loadMoreArenas` (18.6), the identical shape for an identical gap. */
  loadMore: () => void;
}

/**
 * 19.11: fetches `listScheduledTaskRuns` for whichever task id is currently selected, returning
 * `AsyncViewState<ScheduledTaskRun[]>` so the detail view can render it through the same
 * `AsyncBoundary` every other list-shaped fetch in this codebase already uses, instead of a
 * bespoke loading/error shape.
 *
 * `generation` guards against a fast reselect (task A -> task B -> task A) letting a slow
 * in-flight response for a task the reader has since navigated away from land after a newer one
 * already resolved -- the same ref-guarded-by-attempt-number idiom `use-session-logs.ts`'s own
 * `loadFirstPage` established for an identical race (there: session id; here: task id). `loadMore`
 * reads the same `generation` ref (without bumping it) so a page-two response for a task the reader
 * has since navigated away from -- or a task whose history was just `reload()`-ed -- is discarded
 * by the same guard rather than a second, independent one.
 */
export function useScheduledTaskHistory(taskId: string | null): UseScheduledTaskHistoryResult {
  const [state, setState] = useState<AsyncViewState<ScheduledTaskRun[]>>({
    data: undefined, initialLoading: false, refreshing: false, stale: false,
  });
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);
  const generation = useRef(0);

  const load = useCallback(() => {
    if (!taskId) {
      setState({ data: undefined, initialLoading: false, refreshing: false, stale: false });
      setNextCursor(null);
      return;
    }
    const attempt = ++generation.current;
    setState((current) => ({
      data: current.data, initialLoading: current.data === undefined, refreshing: current.data !== undefined, stale: false,
    }));
    void agentService.listScheduledTaskRuns(taskId).then((page) => {
      if (attempt !== generation.current) return;
      setState({ data: page.items, initialLoading: false, refreshing: false, stale: false });
      setNextCursor(page.nextCursor);
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

  const loadMore = useCallback(() => {
    if (!taskId || !nextCursor || loadingMore) return;
    const attempt = generation.current;
    setLoadingMore(true);
    void agentService.listScheduledTaskRuns(taskId, { cursor: nextCursor }).then((page) => {
      if (attempt !== generation.current) return;
      setState((current) => ({ ...current, data: [...(current.data ?? []), ...page.items] }));
      setNextCursor(page.nextCursor);
    }).catch((reason: unknown) => {
      if (attempt !== generation.current) return;
      setState((current) => ({
        ...current,
        error: { kind: "error", message: reason instanceof Error ? reason.message : String(reason), retryable: true },
      }));
    }).finally(() => { setLoadingMore(false); });
  }, [loadingMore, nextCursor, taskId]);

  return { ...state, hasMore: nextCursor !== null, loadingMore, loadMore, reload: load };
}
