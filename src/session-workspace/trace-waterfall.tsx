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
  const [viewportWidth, setViewportWidth] = useState(800);

  useEffect(() => {
    const element = viewportRef.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    // Measured rather than assumed: the axis has to match the box it is drawn in, and that box
    // changes when the drawer opens beside it.
    const observer = new ResizeObserver(([entry]) => {
      setViewportWidth(Math.max(1, Math.round(entry.contentRect.width)));
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const rows = useMemo<TraceRow[]>(() => flattenSpanRows(spans), [spans]);
  const scale = useMemo(
    () => traceTimeScale(spans, viewportWidth, zoom),
    [spans, viewportWidth, zoom],
  );
  const ticks = useMemo(() => traceAxisTicks(scale), [scale]);

  useEffect(() => {
    // Keyboard navigation moves a selection the reader cannot see unless the list follows it. This
    // is the one place automatic scrolling is correct: it is a response to their own key press.
    if (selection.selectedIndex >= 0) {
      listRef.current?.scrollToIndex(selection.selectedIndex, "auto");
    }
  }, [selection.selectedIndex]);

  return (
    <div className="flex min-h-0 flex-1 flex-col" ref={viewportRef}>
      <div
        aria-hidden="true"
        className="grid grid-cols-[minmax(10rem,18rem)_minmax(0,1fr)] gap-2 border-b border-border pb-1 text-[11px] text-muted-foreground"
      >
        <span className="px-1">{t("traces.spanColumn")}</span>
        <div className="relative h-4" style={{ minWidth: scale.contentWidthPx }}>
          {ticks.map((tick, index) => (
            <span
              className="absolute -translate-x-1/2 tabular-nums"
              key={tick}
              style={{ insetInlineStart: `${(index / (ticks.length - 1)) * 100}%` }}
            >
              {t("traces.axisTick", { offset: tick })}
            </span>
          ))}
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
        <div style={{ minWidth: scale.contentWidthPx }}>
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
