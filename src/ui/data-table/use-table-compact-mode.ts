import type { RefObject } from "react";
import { useContainerCompactMode } from "../../hooks/use-container-compact-mode";

/**
 * A real `<table>` stops working as columns get squeezed below their content's natural width —
 * this is a local container-width check (not the page-level `useLayoutTier`), since a table
 * embedded in a narrow sidebar should fall back to cards even inside an otherwise-wide layout.
 */
const COMPACT_MAX_WIDTH = 640;

/** Task 20.1: the ResizeObserver wiring itself now lives in the shared `useContainerCompactMode`
 *  (`src/hooks/use-container-compact-mode.ts`) — this stays a thin, named wrapper so `DataTable`
 *  keeps asking for "is this table compact" rather than a bare width/threshold pair, and so this
 *  table's own 640px decision stays local to the table domain instead of becoming a shared constant. */
export function useTableCompactMode(containerRef: RefObject<HTMLElement | null>): boolean {
  return useContainerCompactMode(containerRef, COMPACT_MAX_WIDTH);
}
