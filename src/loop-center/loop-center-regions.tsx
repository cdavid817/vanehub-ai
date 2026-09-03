import { useTranslation } from "react-i18next";
import type { LoopDefinition, LoopInspectionTarget, LoopRun } from "../types/loop";
import type { HorizontalPaneRegion } from "../ui/destination-layout/regions";
import type { LayoutTier } from "../ui/destination-layout/use-layout-tier";
import { LoopInspector } from "./loop-inspector";
import { LoopNavigation } from "./loop-navigation";

/**
 * Loop Center's own pane bounds for the shared `HorizontalPaneRegion` primitive (17.3) -- chosen
 * to match this destination's own former fixed-grid bounds
 * (`minmax(220px,280px)`/`minmax(260px,340px)`, the CSS this replaces) rather than reusing
 * Sessions' wider `NAVIGATION_WIDTH_BOUNDS`/`INSPECTOR_WIDTH_BOUNDS`
 * (main-layout/workbench-layout-preferences.ts): Loop Center's own nav/inspector content (short
 * definition/run rows; run/limits/workspace fields) never needed that much width, and nothing
 * here requires matching Sessions' numbers exactly -- only its resizing/collapse *mechanism*.
 * `default` is each bound's own former `max`: the width the old `minmax()` track rendered at
 * whenever the flexible middle column had room to spare, which was true at every tier wide enough
 * to show these panes inline at all.
 */
export const LOOP_NAVIGATION_PANE_BOUNDS = { min: 220, max: 280, default: 280 };
export const LOOP_INSPECTOR_PANE_BOUNDS = { min: 260, max: 340, default: 340 };

/**
 * Builds Loop Center's `navigation` region for `DestinationLayout` (17.3): definitions+runs list,
 * unchanged from before this task other than no longer owning its own open/close drawer
 * mechanics -- `DestinationLayoutBody` now supplies the inline/`Sheet` split and resize gutter,
 * this only supplies content, bounds, and current width/open state.
 */
export function useLoopNavigationRegion({
  definitions,
  loading,
  onCreateDefinition,
  onDefinitionChange,
  onEditDefinition,
  onOpenChange,
  onRunChange,
  onWidthChange,
  open,
  runs,
  selectedDefinitionId,
  selectedRunId,
  tier,
  width,
}: {
  definitions: LoopDefinition[];
  loading: boolean;
  onCreateDefinition: () => void;
  onDefinitionChange: (id: string) => void;
  onEditDefinition: () => void;
  onOpenChange: (open: boolean) => void;
  onRunChange: (id: string) => void;
  onWidthChange: (width: number) => void;
  open: boolean;
  runs: LoopRun[];
  selectedDefinitionId: string | null;
  selectedRunId: string | null;
  tier: LayoutTier;
  width: number;
}): HorizontalPaneRegion {
  const { t } = useTranslation();
  // DestinationLayoutBody's own `navigationInline`: at wide/standard, this pane is never a
  // collapsible thing (it was unconditionally visible under the old fixed grid too, with no
  // "collapsed while wide" concept at all) -- only `open`'s Sheet-only meaning at narrower tiers
  // is this component's own to control, via the trigger button/close affordance/Sheet dismissal
  // that only exist once this is no longer inline.
  const navigationInline = tier === "wide" || tier === "standard";
  return {
    content: (
      <LoopNavigation
        definitions={definitions}
        id="loop-navigation-drawer"
        loading={loading}
        onClose={navigationInline ? undefined : () => onOpenChange(false)}
        onCreateDefinition={onCreateDefinition}
        onDefinitionChange={onDefinitionChange}
        onEditDefinition={onEditDefinition}
        onRunChange={onRunChange}
        runs={runs}
        selectedDefinitionId={selectedDefinitionId}
        selectedRunId={selectedRunId}
      />
    ),
    label: t("loops.title"),
    max: LOOP_NAVIGATION_PANE_BOUNDS.max,
    min: LOOP_NAVIGATION_PANE_BOUNDS.min,
    onOpenChange,
    onWidthChange,
    open: navigationInline || open,
    width,
  };
}

/**
 * Builds Loop Center's `inspector` region. Still wraps the pre-existing bespoke `LoopInspector`
 * as-is for now -- swapped for the shared `Inspector` + `loop-iteration` provider in a separate
 * commit (17.3 Piece B), kept apart from this region-mechanics change so each is reviewable on its
 * own.
 */
export function useLoopInspectorRegion({
  loading,
  onInspect,
  onOpenChange,
  onWidthChange,
  open,
  run,
  tier,
  width,
}: {
  loading: boolean;
  onInspect?: (target: LoopInspectionTarget) => void;
  onOpenChange: (open: boolean) => void;
  onWidthChange: (width: number) => void;
  open: boolean;
  run: LoopRun | null;
  tier: LayoutTier;
  width: number;
}): HorizontalPaneRegion {
  const { t } = useTranslation();
  // Inline only at wide (DestinationLayoutBody's own `inspectorInline`) -- narrower than
  // navigation's own cutoff, design.md's "collapse Inspector before Navigation". Same
  // always-visible-when-inline reasoning as `useLoopNavigationRegion` above.
  const inspectorInline = tier === "wide";
  return {
    content: (
      <LoopInspector
        id="loop-inspector-drawer"
        loading={loading}
        onClose={inspectorInline ? undefined : () => onOpenChange(false)}
        onInspect={onInspect}
        run={run}
      />
    ),
    label: t("loops.inspector.title"),
    max: LOOP_INSPECTOR_PANE_BOUNDS.max,
    min: LOOP_INSPECTOR_PANE_BOUNDS.min,
    onOpenChange,
    onWidthChange,
    open: inspectorInline || open,
    width,
  };
}
