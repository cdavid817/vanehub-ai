import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  MeasuredVirtualList,
  type MeasuredVirtualListHandle,
} from "../components/measured-virtual-list";
import type { MessageSpeaker } from "../services/message-speaker";
import type { ExecutionSpanSummary } from "../types/execution-observability";
import { TraceSpanRow, spanSpeaker } from "./trace-span-row";
import {
  flattenSpanRows,
  traceAxisTicks,
  traceTimeScale,
  type TraceRow,
} from "./trace-time-scale";
import type { TraceSelection } from "./use-trace-selection";

/** Row height estimate. Rows are uniform here, so this is exact rather than a guess. */
const ROW_HEIGHT_PX = 28;

/**
 * The scrollable waterfall.
 *
 * Vertically virtualized because a run can have more spans than a browser will render without
 * stuttering, and horizontally scaled because the interesting part of a long run is usually a few
 * hundred milliseconds somewhere in the middle. The two are independent: zooming does not change
 * which rows exist, and scrolling rows does not change the time range.
 */
export function TraceWaterfall({
  selection,
  spans,
  speakers,
  zoom,
}: {
  selection: TraceSelection;
  spans: readonly ExecutionSpanSummary[];
  speakers: Map<string | number, MessageSpeaker>;
  zoom: number;
}) {
  const { t } = useTranslation();
  const listRef = useRef<MeasuredVirtualListHandle>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const axisRef = useRef<HTMLDivElement>(null);
  const [viewportWidth, setViewportWidth] = useState(800);
  const [axisWidth, setAxisWidth] = useState(500);

  useEffect(() => {
    const viewport = viewportRef.current;
    const axis = axisRef.current;
    if (!viewport || !axis || typeof ResizeObserver === "undefined") return;
    // Measured rather than assumed: the axis has to match the box it is drawn in, and that box
    // changes when the drawer opens beside it. The bar column is measured on its own: the name
    // column takes up to 18rem of the row, so scaling the whole viewport width onto the bar
    // column pushed the right end of every bar and the last tick off the visible area.
    // Applied on the next frame: writing state inside the callback re-lays out the axis and the
    // rows, which the observer then reports again in the same frame, and the browser gives up
    // with "ResizeObserver loop completed with undelivered notifications" — in dev, an overlay.
    let frame = 0;
    let pending: { axis?: number; viewport?: number } = {};
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const width = Math.max(1, Math.round(entry.contentRect.width));
        if (entry.target === axis) pending.axis = width;
        else pending.viewport = width;
      }
      if (frame !== 0) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        const next = pending;
        pending = {};
        if (next.axis !== undefined) setAxisWidth(next.axis);
        if (next.viewport !== undefined) setViewportWidth(next.viewport);
      });
    });
    observer.observe(viewport);
    observer.observe(axis);
    return () => {
      observer.disconnect();
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  }, []);

  const rows = useMemo<TraceRow[]>(() => flattenSpanRows(spans), [spans]);
  const scale = useMemo(
    () => traceTimeScale(spans, axisWidth, zoom),
    [spans, axisWidth, zoom],
  );
  const ticks = useMemo(() => traceAxisTicks(scale), [scale]);
  // Rows carry the name column beside the scaled bar column, so the scroll extent is the two
  // together; the bar column alone would clip a zoomed-in run at the right.
  const rowMinWidth = scale.contentWidthPx + Math.max(0, viewportWidth - axisWidth);

  useEffect(() => {
    // Keyboard navigation moves a selection the reader cannot see unless the list follows it. This
    // is the one place automatic scrolling is correct: it is a response to their own key press.
    if (selection.selectedIndex >= 0) {
      listRef.current?.scrollToIndex(selection.selectedIndex, "auto");
    }
  }, [selection.selectedIndex]);

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col" ref={viewportRef}>
      <div
        aria-hidden="true"
        // Same horizontal padding as a row, so the axis column and the bar column are the same
        // width and a bar that ends at the last tick ends at the last tick.
        className="grid grid-cols-[minmax(10rem,18rem)_minmax(0,1fr)] gap-2 border-b border-border px-1 pb-1 text-[11px] text-muted-foreground"
      >
        <span className="px-1">{t("traces.spanColumn")}</span>
        {/* The probe has no min-width of its own, so it reports the column's real width; the
            scaled axis inside it is clipped when zoomed, the same way the rows scroll. */}
        <div className="relative h-4 min-w-0 overflow-hidden" ref={axisRef}>
          <div className="relative h-4" style={{ width: scale.contentWidthPx }}>
            {ticks.map((tick, index) => (
              <span
                // The first and last labels sit flush with the edges instead of centred on them,
                // so neither is half-clipped by the column.
                className={
                  index === 0 ? "absolute whitespace-nowrap tabular-nums" : index === ticks.length - 1 ? "absolute -translate-x-full whitespace-nowrap tabular-nums" : "absolute -translate-x-1/2 whitespace-nowrap tabular-nums"
                }
                key={tick}
                style={{ insetInlineStart: `${(index / (ticks.length - 1)) * 100}%` }}
              >
                {t("traces.axisTick", { offset: tick })}
              </span>
            ))}
          </div>
        </div>
      </div>
      <div
        // One focusable element for the whole list, moved by arrow keys. Tabbing through rows is
        // not an option in a virtualized list: the rows nobody scrolled to are not in the DOM.
        aria-activedescendant={selection.selectedId ?? undefined}
        aria-label={t("traces.waterfall")}
        className="min-h-0 flex-1 overflow-x-auto focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary"
        onKeyDown={(event) => {
          const outcome = selection.handleKey(event.key);
          if (outcome !== null) event.preventDefault();
        }}
        role="application"
        tabIndex={0}
      >
        {/* h-full is load-bearing: the list sizes its viewport from this box, and an auto-height
            box is as tall as the rows the list decides to render — which, on a remount, is none.
            A definite height breaks that loop. */}
        <div className="h-full" style={{ minWidth: rowMinWidth }}>
          <MeasuredVirtualList
            ariaLabel={t("traces.spans")}
            className="h-full"
            estimateSize={() => ROW_HEIGHT_PX}
            getItemKey={(row) => row.span.spanId}
            items={rows}
            overscan={12}
            ref={listRef}
            renderItem={(row) => (
              <TraceSpanRow
                depth={row.depth}
                onSelect={() => selection.select(row.span.spanId)}
                scale={scale}
                selected={row.span.spanId === selection.selectedId}
                span={row.span}
                speaker={spanSpeaker(row.span, speakers)}
              />
            )}
            testId="trace-waterfall-list"
          />
        </div>
      </div>
    </div>
  );
}
