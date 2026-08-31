import { useEffect, useRef, type ReactNode } from "react";
import { DestinationLayoutBody } from "./DestinationLayoutBody";
import type { HorizontalPaneRegion, RuntimePanelRegion } from "./regions";
import { useLayoutTier, type LayoutTier } from "./use-layout-tier";

export interface DestinationLayoutProps {
  navigation?: HorizontalPaneRegion;
  inspector?: HorizontalPaneRegion;
  runtimePanel?: RuntimePanelRegion;
  main: ReactNode;
  className?: string;
  /**
   * The tier is measured inside this component (its own `ResizeObserver`), but a caller building
   * `inspector`/`navigation` region content needs to know it too — e.g. to decide whether that
   * content needs its own visible close affordance, since `Sheet` (used at narrower tiers) has
   * none of its own beyond Escape/backdrop dismiss. Fires from an effect, not during render,
   * since this updates a *different* component's state.
   */
  onTierChange?: (tier: LayoutTier) => void;
}

/** Measures its own container and delegates composition to `DestinationLayoutBody` per tier. */
export function DestinationLayout({ onTierChange, ...props }: DestinationLayoutProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const { tier, width } = useLayoutTier(containerRef);

  useEffect(() => {
    onTierChange?.(tier);
    // `onTierChange` deliberately excluded: this only needs to fire when the tier itself changes,
    // not whenever a caller that didn't memoize its callback happens to re-render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tier]);

  return (
    <div className="h-full min-h-0 min-w-0" ref={containerRef}>
      <DestinationLayoutBody {...props} containerWidth={width} tier={tier} />
    </div>
  );
}
