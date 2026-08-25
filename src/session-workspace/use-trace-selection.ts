import { useCallback, useEffect, useMemo, useState } from "react";

/**
 * Which span row is selected, and how the keyboard moves that selection.
 *
 * A waterfall is a list before it is a picture. Somebody navigating it with a keyboard has to be
 * able to reach every row, and a virtualized list makes that harder rather than easier: the rows
 * they have not scrolled to are not in the DOM, so tabbing through them is not an option at all.
 * The answer is a roving selection — one focusable element, moved by arrow keys — which is why
 * the selection lives here rather than in whichever row happens to be rendered.
 */
export interface TraceSelection {
  /** The selected span, or nothing when the trace is empty. */
  selectedId: string | null;
  selectedIndex: number;
  /** Whether the detail drawer is open. Separate from selection: moving through rows is not
   *  opening them, and a drawer that opened on every arrow key would make the list unusable. */
  detailOpen: boolean;
  select: (spanId: string) => void;
  /** Moves the selection and returns the row that should be scrolled into view. */
  move: (delta: number) => number | null;
  moveTo: (index: number) => number | null;
  openDetail: () => void;
  closeDetail: () => void;
  /** Handles a key event on the list. Returns whether it was consumed. */
  handleKey: (key: string) => number | null | "handled";
}

export function useTraceSelection(spanIds: readonly string[]): TraceSelection {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);

  const ids = useMemo(() => [...spanIds], [spanIds]);
  const selectedIndex = selectedId ? ids.indexOf(selectedId) : -1;

  useEffect(() => {
    // A selection that survived a trace change would name a span this trace does not have, and the
    // drawer would show the previous run's work under the current run's heading.
    if (selectedId && !ids.includes(selectedId)) {
      setSelectedId(ids[0] ?? null);
      setDetailOpen(false);
      return;
    }
    if (!selectedId && ids.length > 0) setSelectedId(ids[0]);
  }, [ids, selectedId]);

  const moveTo = useCallback((index: number): number | null => {
    if (ids.length === 0) return null;
    // Clamped rather than wrapped. Wrapping from the last row to the first is disorienting in a
    // list whose vertical position carries meaning — the reader would jump from the end of the run
    // back to its start with no indication that they had.
    const next = Math.min(ids.length - 1, Math.max(0, index));
    setSelectedId(ids[next]);
    return next;
  }, [ids]);

  const move = useCallback((delta: number): number | null => {
    const from = selectedIndex >= 0 ? selectedIndex : 0;
    return moveTo(from + delta);
  }, [moveTo, selectedIndex]);

  const handleKey = useCallback((key: string): number | null | "handled" => {
    switch (key) {
      case "ArrowDown":
        return move(1);
      case "ArrowUp":
        return move(-1);
      case "Home":
        return moveTo(0);
      case "End":
        return moveTo(ids.length - 1);
      case "PageDown":
        return move(10);
      case "PageUp":
        return move(-10);
      case "Enter":
      case " ":
        setDetailOpen(true);
        return "handled";
      case "Escape":
        // Closes the drawer without moving the selection, so the reader comes back to the row they
        // were looking at rather than to wherever the list started.
        setDetailOpen(false);
        return "handled";
      default:
        return null;
    }
  }, [ids.length, move, moveTo]);

  return {
    selectedId,
    selectedIndex,
    detailOpen,
    select: (spanId: string) => {
      setSelectedId(spanId);
      setDetailOpen(true);
    },
    move,
    moveTo,
    openDetail: () => setDetailOpen(true),
    closeDetail: () => setDetailOpen(false),
    handleKey,
  };
}
