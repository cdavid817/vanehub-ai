import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import type { TraceTransitionNotice } from "../types/trace-transition";

/**
 * How long notices are gathered before one refresh is issued.
 *
 * A busy run emits a transition per span start and per span finish, which for a run doing real
 * work is dozens per second. Refetching on each would put the timeline query into a loop that
 * never settles, and every response would be stale before it rendered. The window is short enough
 * that a reader watching a run sees it move, and long enough that a burst becomes one read.
 */
export const TRACE_REFRESH_WINDOW_MS = 400;

/**
 * Turns a stream of transitions into an occasional refresh.
 *
 * Refreshes, and nothing else. It deliberately does not report "a newer run exists": a notice
 * carries no session id and its run-list flag is set for finishes as well as starts, so anything
 * derived from it here would announce a new run when the reader's own run merely ended, and
 * announce one from a background session over a list that does not contain it. Whether a newer run
 * exists is visible in the refreshed list itself, which is already scoped to the session.
 *
 * Only while the panel is visible. A hidden Traces tab that kept refetching would spend a
 * subscription and a query per transition on a view nobody is looking at — and the moment it
 * becomes visible it re-reads anyway, so nothing is gained by having kept up.
 *
 * Run transitions and span transitions are handled separately because they invalidate different
 * things. A span finishing changes the open timeline; it does not change the list of runs, and
 * re-reading that list once per span is how a busy run makes the whole panel unusable.
 *
 * A settled burst *invalidates* the affected queries rather than changing their keys. The two are
 * not interchangeable: a new key is a different query with no data, so the run list forgets the
 * pages it loaded, the selection correction sees an empty list and clears itself, and the viewport
 * unmounts with the reader's zoom, filters and open drawer inside it. An invalidation refetches
 * the same query and leaves all of that standing.
 */
export function useTraceLiveRefresh({
  isVisible,
  runIds,
  sessionId,
  subscribe,
}: {
  isVisible: boolean;
  /**
   * Every run whose timeline is on screen — the selected one, and the compared one when open.
   *
   * A list rather than a single id because the comparison panel is a second open timeline. When it
   * watched only the selected run, a comparison against a still-advancing run silently showed a
   * snapshot from whenever it was opened, and nothing on screen said so.
   */
  runIds: readonly (string | null)[];
  /** Scopes the run-list invalidation to the session whose list is on screen. */
  sessionId: string | null;
  subscribe: ((listener: (notice: TraceTransitionNotice) => void) => () => void) | null;
}) {
  const queryClient = useQueryClient();
  const pending = useRef<{ timelines: Set<string>; runList: boolean }>({
    timelines: new Set(),
    runList: false,
  });
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const watched = useRef<readonly (string | null)[]>(runIds);
  watched.current = runIds;
  const currentSessionId = useRef(sessionId);
  currentSessionId.current = sessionId;

  useEffect(() => {
    if (!isVisible || !subscribe) return;

    const flush = () => {
      timer.current = null;
      const owed = pending.current;
      pending.current = { timelines: new Set(), runList: false };
      for (const settledRunId of owed.timelines) {
        void queryClient.invalidateQueries({
          queryKey: ["execution-timeline", settledRunId],
        });
      }
      if (owed.runList) {
        void queryClient.invalidateQueries({
          queryKey: ["execution-runs", currentSessionId.current],
        });
      }
    };

    const release = subscribe((notice) => {
      if (notice.affectsRunList) pending.current.runList = true;
      // A span transition in a run nobody is looking at changes nothing on screen. Refetching for
      // it would let one busy background run keep a reader's open timeline in permanent motion.
      if (watched.current.includes(notice.runId)) pending.current.timelines.add(notice.runId);
      if (!pending.current.timelines.size && !pending.current.runList) return;
      // Trailing rather than leading: the last transition in a burst is the one whose state the
      // refetch should return, and a leading edge would read the corpus as it was before the
      // burst started.
      if (timer.current === null) {
        timer.current = setTimeout(flush, TRACE_REFRESH_WINDOW_MS);
      }
    });

    return () => {
      release();
      if (timer.current !== null) {
        clearTimeout(timer.current);
        timer.current = null;
      }
      // What was owed is dropped along with the subscription. Becoming visible again re-reads
      // from scratch, so carrying a stale debt across would only produce a duplicate read.
      pending.current = { timelines: new Set(), runList: false };
    };
  }, [isVisible, queryClient, subscribe]);

}
