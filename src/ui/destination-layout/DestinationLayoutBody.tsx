import type { ReactNode } from "react";
import { Sheet } from "../sheet/Sheet";
import { SplitPane } from "../split-pane/SplitPane";
import type { HorizontalPaneRegion, RuntimePanelRegion } from "./regions";
import type { LayoutTier } from "./use-layout-tier";

/**
 * Not stated as a pixel value in design.md Decision 3 ("主工作面设置最小可读宽度") — chosen as a
 * floor below which the work surface reads as unusable, same order of magnitude as an editor
 * pane's practical minimum. Only enforced at the `wide` tier, where navigation and inspector
 * compete for the same row; narrower tiers keep at most one pane inline already.
 */
export const MAIN_MIN_WIDTH = 320;

export interface DestinationLayoutBodyProps {
  tier: LayoutTier;
  containerWidth: number;
  navigation?: HorizontalPaneRegion;
  inspector?: HorizontalPaneRegion;
  runtimePanel?: RuntimePanelRegion;
  main: ReactNode;
  className?: string;
}

/**
 * Pure composition given an already-classified tier — kept separate from `DestinationLayout`'s
 * `ResizeObserver` wrapper so tests can exercise every tier directly. jsdom does not implement
 * `ResizeObserver`, and this repo's existing tests stub it as a no-op observer that never fires
 * (see `shell-tab.test.tsx`), which would otherwise leave a mounted layout stuck at its default
 * tier forever.
 */
export function DestinationLayoutBody({ tier, containerWidth, navigation, inspector, runtimePanel, main, className }: DestinationLayoutBodyProps) {
  const navigationInline = tier === "wide" || tier === "standard";
  const inspectorInline = tier === "wide";

  // At `wide`, both panes compete with the work surface for the same row. If keeping both open
  // would starve it below MAIN_MIN_WIDTH, inspector yields first (design.md: "优先折叠 Inspector,
  // 再折叠 Navigation") — this only affects what renders this frame, not the user's stored `open`
  // preference, so the pane reappears on its own once the window widens again.
  const bothInlineOpen = inspectorInline && navigationInline && Boolean(navigation?.open) && Boolean(inspector?.open);
  const wouldStarveMain = bothInlineOpen && containerWidth - (navigation?.width ?? 0) - (inspector?.width ?? 0) < MAIN_MIN_WIDTH;
  const inspectorInlineOpen = inspectorInline && Boolean(inspector?.open) && !wouldStarveMain;
  const navigationInlineOpen = navigationInline && Boolean(navigation?.open);

  // Compact/narrow demote both to sheets; only one can cover the work surface at a time, so a
  // freshly opened inspector takes priority over navigation left open from a wider layout.
  const inspectorSheetOpen = !inspectorInline && Boolean(inspector?.open);
  const navigationSheetOpen = !navigationInline && Boolean(navigation?.open) && !inspectorSheetOpen;

  let workRow: ReactNode = withRuntimePanel(main, runtimePanel);

  // Wrapped whenever the region exists at all, regardless of tier or open state: `SplitPane`'s
  // `open` prop (not this condition) is what hides the pane, because that keeps `workRow`'s
  // wrapper at the same position in `SplitPane`'s own children no matter what changes around it.
  // Gating this condition on tier or open/closed instead would swap `workRow` between being
  // `SplitPane`'s direct return and being nested inside one, and React remounts a subtree whose
  // ancestor type changed like that — which used to cost a reader an in-progress draft the moment
  // they opened the inspector, and separately (since a hidden container's `ResizeObserver` reports
  // a momentary zero width, which classifies as `narrow` before the real size arrives) the moment
  // they navigated to this destination and back.
  if (inspector) {
    workRow = (
      <SplitPane
        // Whichever of these two ends up outermost is a flex item of this component's own
        // returned row below and needs to grow into it; nested inside the other one's flexRegion
        // wrapper (a plain block, not a flex container), `flex-1` has no effect either way, so
        // this is safe unconditionally rather than only on whichever one that turns out to be.
        className="flex-1"
        direction="row"
        gutterLabel={inspector.label}
        max={inspector.max}
        min={inspector.min}
        onResizeEnd={inspector.onWidthCommit}
        onSizeChange={inspector.onWidthChange}
        open={inspectorInlineOpen}
        primary={workRow}
        resizedPane="secondary"
        secondary={inspector.content}
        size={inspector.width}
      />
    );
  }
  if (navigation) {
    workRow = (
      <SplitPane
        className="flex-1"
        direction="row"
        gutterLabel={navigation.label}
        max={navigation.max}
        min={navigation.min}
        onResizeEnd={navigation.onWidthCommit}
        onSizeChange={navigation.onWidthChange}
        open={navigationInlineOpen}
        primary={navigation.content}
        resizedPane="primary"
        secondary={workRow}
        size={navigation.width}
      />
    );
  }

  return (
    <div className="relative flex h-full min-h-0 min-w-0" data-layout-tier={tier}>
      <div className={className ?? "flex h-full min-h-0 w-full min-w-0"}>{workRow}</div>
      {navigationSheetOpen && navigation ? (
        <Sheet onClose={() => navigation.onOpenChange(false)} placement={tier === "narrow" ? "full" : "left"} title={navigation.label}>
          {navigation.content}
        </Sheet>
      ) : null}
      {inspectorSheetOpen && inspector ? (
        <Sheet onClose={() => inspector.onOpenChange(false)} placement={tier === "narrow" ? "full" : "right"} title={inspector.label}>
          {inspector.content}
        </Sheet>
      ) : null}
    </div>
  );
}

/**
 * Wrapped whenever the region exists at all, mirroring `inspector`/`navigation` above for the same
 * reason: gating this on `.open` instead would swap `main` between being this function's direct
 * return and being nested inside a freshly-mounted `SplitPane`, and React remounts a subtree whose
 * ancestor type changed like that — the caller must keep passing a `runtimePanel` object across the
 * open/close toggle (only flipping `.open`), not swap the whole prop to `undefined` when closed, or
 * the remount just moves one level up.
 */
function withRuntimePanel(main: ReactNode, runtimePanel: RuntimePanelRegion | undefined): ReactNode {
  if (!runtimePanel) return main;
  return (
    <SplitPane
      direction="column"
      gutterLabel={runtimePanel.label}
      max={runtimePanel.max}
      min={runtimePanel.min}
      onResizeEnd={runtimePanel.onHeightCommit}
      onSizeChange={runtimePanel.onHeightChange}
      open={runtimePanel.open}
      primary={main}
      resizedPane="secondary"
      secondary={runtimePanel.content}
      size={runtimePanel.height}
    />
  );
}
