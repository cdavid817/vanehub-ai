import { useEffect, useRef, useState } from "react";
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

export interface TraceLiveRefresh {
  /** Bumped once per settled burst. A query keyed on it refetches; one that is not, does not. */
  refreshToken: number;
  /** Bumped when the run list itself should be re-read, which is rarer. */
  runListToken: number;
}

/**
 * Turns a stream of transitions into an occasional refresh.
 *
 * Only while the panel is visible. A hidden Traces tab that kept refetching would spend a
 * subscription and a query per transition on a view nobody is looking at — and the moment it
 * becomes visible it re-reads anyway, so nothing is gained by having kept up.
 *
 * Run transitions and span transitions are counted separately because they invalidate different
 * things. A span finishing changes the open timeline; it does not change the list of runs, and
 * re-reading that list once per span is how a busy run makes the whole panel unusable.
 */
export function useTraceLiveRefresh({
  isVisible,
  runId,
  subscribe,
}: {
  isVisible: boolean;
  /** The run currently open. Transitions for other runs never touch the timeline token. */
  runId: string | null;
  subscribe: ((listener: (notice: TraceTransitionNotice) => void) => () => void) | null;
}): TraceLiveRefresh {
  const [refreshToken, setRefreshToken] = useState(0);
  const [runListToken, setRunListToken] = useState(0);
  const pending = useRef<{ timeline: boolean; runList: boolean }>({
    timeline: false,
    runList: false,
  });
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const currentRunId = useRef(runId);
  currentRunId.current = runId;

  useEffect(() => {
    if (!isVisible || !subscribe) return;

    const flush = () => {
      timer.current = null;
      const owed = pending.current;
      pending.current = { timeline: false, runList: false };
      if (owed.timeline) setRefreshToken((token) => token + 1);
      if (owed.runList) setRunListToken((token) => token + 1);
    };

    const release = subscribe((notice) => {
      if (notice.affectsRunList) pending.current.runList = true;
      // A span transition in another run changes nothing this view is showing. Refetching for it
      // would make one busy background run keep a reader's open timeline in permanent motion.
      if (notice.runId === currentRunId.current) pending.current.timeline = true;
      if (!pending.current.timeline && !pending.current.runList) return;
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
      pending.current = { timeline: false, runList: false };
    };
  }, [isVisible, subscribe]);

  return { refreshToken, runListToken };
}
