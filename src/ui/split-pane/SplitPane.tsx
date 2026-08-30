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

  return (
    <div className={cn("flex min-h-0 min-w-0", direction === "row" ? "flex-row" : "flex-col", className)}>
      {startIsSized ? sizedRegion(primary) : flexRegion(primary)}
      <div
        aria-label={gutterLabel}
        aria-orientation={direction === "row" ? "vertical" : "horizontal"}
        aria-valuemax={max}
        aria-valuemin={min}
        aria-valuenow={clamped}
        className={cn(
          "ucd-focus-ring shrink-0 touch-none bg-border-subtle hover:bg-accent",
          direction === "row" ? "w-2 cursor-col-resize" : "h-2 cursor-row-resize",
        )}
        onKeyDown={handleKeyDown}
        onPointerDown={handlePointerDown}
        role="separator"
        tabIndex={0}
      />
      {startIsSized ? flexRegion(secondary) : sizedRegion(secondary)}
    </div>
  );
}
