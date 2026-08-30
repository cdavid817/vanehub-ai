import { useEffect, useState, type RefObject } from "react";

export type LayoutTier = "wide" | "standard" | "compact" | "narrow";

/**
 * Breakpoints are not stated explicitly in design.md Decision 3 — derived from the required
 * screenshot matrix (Decision 20: widths 1600/1280/1024/768/640), which lines up with Tailwind's
 * own lg/xl scale. Placing tier boundaries at those exact values means the matrix tests each
 * width squarely inside a tier plus the two above/below a transition, not straddling one.
 */
const TIER_MIN_WIDTH: { tier: LayoutTier; minWidth: number }[] = [
  { tier: "wide", minWidth: 1280 },
  { tier: "standard", minWidth: 1024 },
  { tier: "compact", minWidth: 768 },
  { tier: "narrow", minWidth: 0 },
];

export function classifyLayoutTier(width: number): LayoutTier {
  return TIER_MIN_WIDTH.find((entry) => width >= entry.minWidth)?.tier ?? "narrow";
}

/**
 * Returns both the tier and the raw width from the same ResizeObserver callback. A component
 * that only re-rendered on tier changes would read a stale width from the ref on every resize
 * that stays within one tier — `width` needs its own state, not a ref read during render.
 */
export function useLayoutTier(containerRef: RefObject<HTMLElement | null>): { tier: LayoutTier; width: number } {
  const [state, setState] = useState<{ tier: LayoutTier; width: number }>({ tier: "wide", width: Number.POSITIVE_INFINITY });

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? element.clientWidth;
      setState({ tier: classifyLayoutTier(width), width });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [containerRef]);

  return state;
}
