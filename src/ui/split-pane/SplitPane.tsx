import type { ReactNode } from "react";
import { cn } from "../../lib/utils";
import { clampSize, usePaneResize } from "./use-pane-resize";

export type SplitPaneDirection = "row" | "column";

export interface SplitPaneProps {
  direction: SplitPaneDirection;
  /** Always renders in the start (left/top) position. */
  primary: ReactNode;
  /** Always renders in the end (right/bottom) position. */
  secondary: ReactNode;
  /** Which pane the `size`/gutter controls; the other pane fills the remaining space. */
  resizedPane?: "primary" | "secondary";
  /**
   * Renders only the unsized side's own wrapper, with neither the gutter nor `secondary` —
   * not simply `size={0}`, which `min` would clamp back up. Defaults to `true` so every existing
   * caller is unaffected.
   *
   * That wrapper is the same element `primary`/`secondary` sits in either way (only its siblings
   * change), which is what keeps this side's content mounted across an open/close toggle: a
   * caller that skipped rendering `SplitPane` entirely instead, swapping it for a bare
   * `resizedPane`-side element, would change that element's ancestor one level up and React would
   * remount its subtree — exactly the bug this option exists to avoid (destination-layout wraps
   * `main` this way, and toggling the inspector open used to wipe an in-progress draft under it).
   */
  open?: boolean;
  size: number;
  onSizeChange: (size: number) => void;
  /** Fires once per drag/keypress commit — the moment a caller should persist the size. */
  onResizeEnd?: (size: number) => void;
  min: number;
  max: number;
  gutterLabel: string;
  className?: string;
}

/**
 * Two-region resizable layout (design.md Decision 3's pane model). Owns no persistence itself —
 * `src/ui/` cannot reach a settings service — callers wire `onResizeEnd` to their own storage.
 */
export function SplitPane({
  direction,
  primary,
  secondary,
  resizedPane = "primary",
  open = true,
  size,
  onSizeChange,
  onResizeEnd,
  min,
  max,
  gutterLabel,
  className,
}: SplitPaneProps) {
  const clamped = clampSize(size, min, max);
  const startIsSized = resizedPane === "primary";
  const { isDragging, handlePointerDown, handleKeyDown } = usePaneResize({
    direction,
    growsWithPointer: startIsSized ? 1 : -1,
    size: clamped,
    min,
    max,
    onSizeChange,
    onResizeEnd,
  });

  function sizedRegion(content: ReactNode) {
    const sizedStyle = { [direction === "row" ? "width" : "height"]: clamped };
    return (
      <div className={cn("min-h-0 min-w-0 overflow-hidden", !isDragging && "ucd-pane-transition")} style={sizedStyle}>
        {content}
      </div>
    );
  }

  function flexRegion(content: ReactNode) {
    return <div className="min-h-0 min-w-0 flex-1 overflow-hidden">{content}</div>;
  }

  // The flex-side wrapper always renders at the same position in this element's children,
  // whichever side that is — that positional stability, not the wrapper's specific div, is what
  // lets a caller open/close the other side without React reconciling its content as a fresh
  // subtree (see the `open` prop's doc comment).
  // `h-full`, not just `flex-1`/`min-h-0`: those only size this element as a flex item of a flex
  // parent. A `SplitPane` nested inside another `SplitPane`'s own flex-side wrapper sits in a
  // plain block div instead (see that wrapper below), where block-level height defaults to
  // content size — `h-full` is what makes this element inherit its parent's real height there too.
  return (
    <div className={cn("flex h-full min-h-0 min-w-0", direction === "row" ? "flex-row" : "flex-col", className)}>
      {startIsSized ? (open ? sizedRegion(primary) : null) : flexRegion(primary)}
      {open ? (
        <div
          aria-label={gutterLabel}
          aria-orientation={direction === "row" ? "vertical" : "horizontal"}
          aria-valuemax={max}
          aria-valuemin={min}
          aria-valuenow={clamped}
          className={cn(
            // 20.6: the painted bar stays thin (w-2/h-2) so it doesn't eat pane space, but a
            // resize handle is still a pointer target -- `before:` widens the invisible hit area
            // to the same ~44px minimum other controls use, via a slop pseudo-element rather than
            // real layout size (a real 44px gutter would visibly steal that much width/height from
            // whichever pane sits next to it). Pointer/keyboard handlers stay on this element, so
            // a pointerdown anywhere in that painted-but-invisible slop still resolves to this same
            // node -- pseudo-elements are not part of the DOM and always report their host as the
            // event target.
            //
            // A slop reaching equally far into *both* neighboring panes (a symmetric -18px/-18px
            // on every side) regressed real e2e clicks twice, not hypothetically: `before:` is
            // `position:absolute` with a positive `z-index`, and CSS's stacking order always
            // paints a positioned+z-indexed box above plain in-flow siblings regardless of DOM
            // order or z-index magnitude (removing `z-10` would not fix this -- an auto-z
            // positioned box still outranks non-positioned in-flow content), so the slop's own
            // painted pixels intercept pointer events meant for whatever real content happens to
            // sit in that band. Confirmed live both times via the actual Playwright call log, not
            // reasoned about abstractly: `direction="column"`'s one production caller
            // (`DestinationLayoutBody.tsx`'s Runtime Panel wrapper) has its secondary pane's header
            // row (tabs, maximize, close) flush against the gutter with a measured 0px gap
            // (`boundingBox()`: gutter bottom and header top share the same y; the header's own
            // controls, centered in a 33.5px row, land ~17px below that -- squarely inside an 18px
            // downward slop). `direction="row"`'s Navigation gutter has the same shape on its own
            // secondary side: Terminal History's view-filter tabs (`execution-record-toolbar.tsx`)
            // render flush against the left edge of whichever pane sits right of the sidebar, so an
            // 18px rightward slop swallows their clicks too.
            //
            // Extending into the *primary* side never showed this in a full e2e sweep, so it keeps
            // 18px there on both axes. The *secondary* side is capped flush with the bar's own edge
            // (0) instead of reaching 18px further in, on both axes -- trading a perfect symmetric
            // 44px for an honest 26px (18 + ~8 bar + 0) that cannot re-swallow a sibling pane's real
            // controls. Short of the 44px target on that one side, kept that way deliberately rather
            // than reintroduced.
            "ucd-focus-ring relative shrink-0 touch-none bg-border-subtle hover:bg-accent before:absolute before:z-10 before:content-['']",
            direction === "row"
              ? "w-2 cursor-col-resize before:inset-y-0 before:-left-[18px] before:right-0"
              : "h-2 cursor-row-resize before:inset-x-0 before:-top-[18px] before:bottom-0",
          )}
          onKeyDown={handleKeyDown}
          onPointerDown={handlePointerDown}
          role="separator"
          tabIndex={0}
        />
      ) : null}
      {startIsSized ? flexRegion(secondary) : (open ? sizedRegion(secondary) : null)}
    </div>
  );
}
