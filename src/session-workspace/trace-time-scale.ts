import type { ExecutionSpanSummary } from "../types/execution-observability";

/**
 * Where a span's bar goes, and when it has no business being drawn at all.
 *
 * Pure, and separated from the view for one reason: the interesting cases here are the ones where
 * a bar must *not* be drawn in the obvious place. A span that could not be placed on the timeline
 * has no offset, and rendering it at zero would put work at the start of the run that did not
 * happen there. A span that is still running has no duration, and drawing it to the right edge
 * would show a measurement nobody made. Both are one `?? 0` away from being silently wrong, and
 * neither is visible from a screenshot.
 */

/** How much the reader has zoomed in. 1 fits the whole run; higher spreads it out. */
export const MIN_ZOOM = 1;
export const MAX_ZOOM = 64;

/**
 * Width of the span-label column, and the gap after it.
 *
 * A fixed width rather than `minmax(10rem,18rem)`, because the axis width has to be *derived* from
 * it and a range cannot be subtracted. The range was also the bug: a `minmax` whose maximum is a
 * length absorbs free space ahead of a `1fr` sibling, so in a narrow container the labels took the
 * whole row and the axis column resolved to zero — every bar was rendered, correctly sized, into a
 * column with no width.
 *
 * Shared with the row component so the header ticks and the bars below them cannot disagree about
 * where the axis starts.
 */
export const LABEL_COLUMN_PX = 208;
export const LABEL_COLUMN_GAP_PX = 8;

/** Horizontal padding each row carries (`px-1` on both sides), which the axis does not get. */
const ROW_PADDING_PX = 8;

/**
 * The axis width available inside a viewport of `viewportWidthPx`.
 *
 * Subtracts the row padding as well as the label column: a full-run bar computed against the
 * unpadded width overshoots its own track by exactly that much, which reads as a run whose last
 * span continues past the end of the axis.
 */
export function axisWidthFor(viewportWidthPx: number): number {
  return Math.max(1, viewportWidthPx - LABEL_COLUMN_PX - LABEL_COLUMN_GAP_PX - ROW_PADDING_PX);
}

/**
 * The scrollable content width that gives an axis of `contentWidthPx` exactly that much room.
 *
 * The inverse of `axisWidthFor`, and it has to stay that way. When the two disagree the padding is
 * subtracted twice — once from the axis and once from the row it sits in — and the widest bar
 * overshoots its track by that difference at every viewport too narrow to avoid scrolling.
 */
export function contentMinWidthFor(contentWidthPx: number): number {
  return LABEL_COLUMN_PX + LABEL_COLUMN_GAP_PX + ROW_PADDING_PX + contentWidthPx;
}

/**
 * The narrowest a bar may render.
 *
 * A span that took under a millisecond would otherwise be invisible, which reads as "it did not
 * happen" rather than "it was fast". The floor is in pixels rather than milliseconds because it is
 * about being seeable, not about being long.
 */
export const MIN_BAR_WIDTH_PX = 3;

export interface TraceTimeScale {
  /** Total run duration the axis covers, in milliseconds. */
  totalMs: number;
  /** Width of the scrollable content at the current zoom. */
  contentWidthPx: number;
  zoom: number;
}

/**
 * What a bar's width means.
 *
 * Three values rather than a boolean, because an absent duration has two different causes that a
 * reader must not confuse. The native projection omits a duration when the span is still going —
 * and also when it ended but its timestamps could not be subtracted. Collapsing those into one
 * flag reports a failed span as running work.
 */
export type SpanBarMeasurement =
  /** The span ended and its duration was derived from its own timestamps. */
  | "measured"
  /** The span has not ended. The bar runs to the axis end because where it stops has not happened. */
  | "running"
  /** The span ended, but no duration could be derived. The bar is bounded and says nothing more. */
  | "unknown";

/** Where one span's bar sits, or why it has none. */
export type SpanBarPlacement =
  | { kind: "placed"; leftPx: number; widthPx: number; measurement: SpanBarMeasurement }
  /** The span carries no offset, so nothing here can say where it belongs. */
  | { kind: "unplaceable" };

/**
 * The statuses that mean the work stopped.
 *
 * `incomplete` belongs here: it says the work ended without a verified result, which is a
 * different thing from still producing one.
 */
const TERMINAL_STATUSES: ReadonlySet<ExecutionSpanSummary["status"]> = new Set([
  "succeeded",
  "failed",
  "cancelled",
  "incomplete",
]);

/**
 * What can be said about one span's duration.
 *
 * Exported and shared so the bar, the label, and the detail surface cannot disagree. They render
 * the same fact three ways, and the defect this replaces was each of them deriving it separately
 * from the one input that cannot distinguish the cases.
 */
export function spanMeasurement(span: ExecutionSpanSummary): SpanBarMeasurement {
  if (span.completedDurationMs !== undefined) return "measured";
  return TERMINAL_STATUSES.has(span.status) ? "unknown" : "running";
}

/**
 * Builds a scale from the run's own span set.
 *
 * The axis length comes from the spans rather than from the run's `durationMs`, because a running
 * run has none — and an axis that collapsed to zero the moment a run was live would make the
 * waterfall useless exactly while it is most worth watching.
 */
export function traceTimeScale(
  spans: readonly ExecutionSpanSummary[],
  viewportWidthPx: number,
  zoom: number,
): TraceTimeScale {
  const clampedZoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
  const furthest = spans.reduce((longest, span) => {
    const offset = span.startOffsetMs;
    if (offset === undefined) return longest;
    // A running span contributes only where it started. Adding an assumed length would put the
    // axis end somewhere nothing has been observed to reach.
    return Math.max(longest, offset + (span.completedDurationMs ?? 0));
  }, 0);
  return {
    // Never zero: a zero-length axis makes every division undefined, and the first frame of a run
    // legitimately has one span at offset zero with no duration yet.
    totalMs: Math.max(furthest, 1),
    contentWidthPx: Math.max(viewportWidthPx, 1) * clampedZoom,
    zoom: clampedZoom,
  };
}

/**
 * Places one span, or refuses to.
 *
 * The measurement travels with the placement rather than being inferred from the width, so a
 * renderer cannot accidentally draw a definite end on a span that has none — or an open end on one
 * that stopped. Which of those applies is decided from status and duration together, because
 * duration alone cannot tell "not finished" from "finished, not measurable".
 */
export function placeSpanBar(
  span: ExecutionSpanSummary,
  scale: TraceTimeScale,
): SpanBarPlacement {
  const offset = span.startOffsetMs;
  if (offset === undefined) return { kind: "unplaceable" };

  const pxPerMs = scale.contentWidthPx / scale.totalMs;
  const leftPx = offset * pxPerMs;
  const measurement = spanMeasurement(span);
  if (measurement === "running") {
    // Runs to the end of the axis, and says so. The width is a placeholder for "still going", not
    // a measurement.
    return {
      kind: "placed",
      leftPx,
      widthPx: Math.max(MIN_BAR_WIDTH_PX, scale.contentWidthPx - leftPx),
      measurement,
    };
  }
  if (measurement === "unknown") {
    // Bounded at the minimum width: the span stopped, so the bar must not run to the edge, but no
    // length here would be a measurement anybody made. The flag is what carries the meaning.
    return { kind: "placed", leftPx, widthPx: MIN_BAR_WIDTH_PX, measurement };
  }
  return {
    kind: "placed",
    leftPx,
    widthPx: Math.max(MIN_BAR_WIDTH_PX, (span.completedDurationMs ?? 0) * pxPerMs),
    measurement,
  };
}

/**
 * Evenly spaced tick marks along the axis, as millisecond offsets.
 *
 * Bounded rather than proportional to the zoom: past a certain density the labels overlap into an
 * unreadable band, and a reader cannot tell an axis with too many ticks from one that is broken.
 */
export function traceAxisTicks(scale: TraceTimeScale, maxTicks = 8): number[] {
  const count = Math.max(2, Math.min(maxTicks, Math.round(scale.zoom) + 1));
  return Array.from({ length: count }, (_unused, index) =>
    Math.round((scale.totalMs / (count - 1)) * index),
  );
}

/**
 * Flattens the span tree into rows, depth-first, with the depth each row renders at.
 *
 * Depth-first because that is the order a reader follows a trace: a span, then what it caused,
 * then what came after it. A breadth-first list would put a span's siblings between it and its
 * own children.
 */
export interface TraceRow {
  span: ExecutionSpanSummary;
  depth: number;
}

export function flattenSpanRows(spans: readonly ExecutionSpanSummary[]): TraceRow[] {
  const children = new Map<string, ExecutionSpanSummary[]>();
  const roots: ExecutionSpanSummary[] = [];
  const known = new Set(spans.map((span) => span.spanId));
  for (const span of spans) {
    const parent = span.parentSpanId;
    // A span whose parent is not in this set is a root here. Hiding it because its parent was
    // filtered out or never recorded would drop real work from the list entirely.
    if (parent && parent !== span.spanId && known.has(parent)) {
      children.set(parent, [...(children.get(parent) ?? []), span]);
    } else {
      roots.push(span);
    }
  }

  const rows: TraceRow[] = [];
  const visited = new Set<string>();
  const walk = (span: ExecutionSpanSummary, depth: number) => {
    // A cycle is a producer bug that arrives as data. Stopping is what keeps it from becoming an
    // infinite list rather than a wrong one.
    if (visited.has(span.spanId)) return;
    visited.add(span.spanId);
    rows.push({ span, depth });
    for (const child of children.get(span.spanId) ?? []) walk(child, depth + 1);
  };
  for (const root of roots) walk(root, 0);
  // Anything a cycle kept out of the walk still belongs in the list; it is real work, and a
  // malformed parent chain is not a reason to hide it.
  for (const span of spans) {
    if (!visited.has(span.spanId)) rows.push({ span, depth: 0 });
  }
  return rows;
}
