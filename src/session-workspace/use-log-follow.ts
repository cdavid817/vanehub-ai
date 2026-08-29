import { useCallback, useState } from "react";

/**
 * Whether the viewport is allowed to move on its own, and how many rows it owes the reader.
 *
 * A log list that always jumps to the newest row is unusable the moment anything is actually being
 * logged: the line someone is reading scrolls away mid-sentence. A log list that never jumps is
 * useless for watching. So the answer is neither, and this is where the "neither" is decided.
 *
 * Two things independently stop automatic movement, and they are separate because they mean
 * different things. **Paused** is a choice — the reader pressed Pause and expects the view to stay
 * put until they say otherwise. **Scrolled away** is an inference — they dragged the scrollbar, and
 * the fact that they are no longer at the newest edge is the only evidence of intent there is.
 * Collapsing them would make one silently undo the other: resuming from a pause would be cancelled
 * by wherever the scrollbar happened to be, and scrolling back to the top would silently un-pause
 * a reader who explicitly asked for stillness.
 */
export interface LogFollowState {
  /** Whether the viewport may move to the newest row by itself right now. */
  following: boolean;
  /** The reader's explicit choice, independent of where the viewport is. */
  paused: boolean;
  atNewestEdge: boolean;
  /**
   * Rows that arrived while the viewport was held still.
   *
   * The count is what makes not-following honest. Without it a paused reader sees a list that
   * quietly stops matching the session, with nothing to say that more exists just above.
   */
  pendingCount: number;
  setPaused: (paused: boolean) => void;
  /** Records where the viewport is now. Called by the list, not by a timer. */
  noteViewport: (atNewestEdge: boolean) => void;
  /** Announces rows that arrived. Counted only while the viewport is held still. */
  notePending: (count: number) => void;
  /** Resumes following and clears what was owed. The caller does the scrolling. */
  resumeAtNewest: () => void;
}

export function useLogFollow(): LogFollowState {
  const [paused, setPausedState] = useState(false);
  const [atNewestEdge, setAtNewestEdge] = useState(true);
  const [pendingCount, setPendingCount] = useState(0);

  const following = !paused && atNewestEdge;

  const setPaused = useCallback((next: boolean) => {
    setPausedState(next);
    // Un-pausing clears the backlog because the caller is about to show it. Leaving the count up
    // would report rows the reader can already see.
    if (!next) setPendingCount(0);
  }, []);

  const noteViewport = useCallback((next: boolean) => {
    setAtNewestEdge(next);
    // Arriving back at the newest edge means the rows that were owed are now on screen. This does
    // not un-pause: a paused reader who scrolls to the top is still paused, and the next row that
    // arrives must not move their viewport.
    if (next) setPendingCount(0);
  }, []);

  const notePending = useCallback((count: number) => {
    if (count <= 0) return;
    setPendingCount((current) => current + count);
  }, []);

  const resumeAtNewest = useCallback(() => {
    setPausedState(false);
    setAtNewestEdge(true);
    setPendingCount(0);
  }, []);

  return {
    following,
    paused,
    atNewestEdge,
    // Only meaningful while the view is held still. Reported as zero otherwise so a caller does not
    // have to remember to check `following` before rendering it.
    pendingCount: following ? 0 : pendingCount,
    setPaused,
    noteViewport,
    notePending,
    resumeAtNewest,
  };
}
