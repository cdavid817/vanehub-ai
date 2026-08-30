import { useEffect, useState, type RefObject } from "react";

/**
 * A real `<table>` stops working as columns get squeezed below their content's natural width —
 * this is a local container-width check (not the page-level `useLayoutTier`), since a table
 * embedded in a narrow sidebar should fall back to cards even inside an otherwise-wide layout.
 */
const COMPACT_MAX_WIDTH = 640;

export function useTableCompactMode(containerRef: RefObject<HTMLElement | null>): boolean {
  const [compact, setCompact] = useState(false);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? element.clientWidth;
      setCompact(width < COMPACT_MAX_WIDTH);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [containerRef]);

  return compact;
}
