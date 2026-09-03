import { useEffect, useState, type RefObject } from "react";

/**
 * Task 20.1: the shared ResizeObserver-based container-width check behind `useTableCompactMode`
 * (`src/ui/data-table/use-table-compact-mode.ts`) and Mission Control's section-nav compact switch
 * (`src/mission-control/mission-control-section-nav.tsx`) -- both used to independently define this
 * same observer wiring (subscribe, read `contentRect.width` with a `clientWidth` fallback, flip a
 * boolean, disconnect on unmount). Extracted so a third container-width consumer never has to choose
 * between copy-pasting it again or reaching into one of those two files.
 *
 * The compact *threshold* deliberately stays each caller's own value, not a shared constant: a
 * table's readable column width and a nine-tab strip's readable label width are different design
 * decisions that happen to currently agree on 640px, not the same underlying need -- so this hook
 * takes `threshold` as a required argument rather than defaulting it.
 */
export function useContainerCompactMode<T extends HTMLElement>(
  containerRef: RefObject<T | null>,
  threshold: number,
): boolean {
  const [compact, setCompact] = useState(false);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? element.clientWidth;
      setCompact(width < threshold);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [containerRef, threshold]);

  return compact;
}
