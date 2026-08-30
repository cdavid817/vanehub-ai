import { useRef, type ReactNode } from "react";
import { DestinationLayoutBody } from "./DestinationLayoutBody";
import type { HorizontalPaneRegion, RuntimePanelRegion } from "./regions";
import { useLayoutTier } from "./use-layout-tier";

export interface DestinationLayoutProps {
  navigation?: HorizontalPaneRegion;
  inspector?: HorizontalPaneRegion;
  runtimePanel?: RuntimePanelRegion;
  main: ReactNode;
  className?: string;
}

/** Measures its own container and delegates composition to `DestinationLayoutBody` per tier. */
export function DestinationLayout(props: DestinationLayoutProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const { tier, width } = useLayoutTier(containerRef);

  return (
    <div className="h-full min-h-0 min-w-0" ref={containerRef}>
      <DestinationLayoutBody {...props} containerWidth={width} tier={tier} />
    </div>
  );
}
