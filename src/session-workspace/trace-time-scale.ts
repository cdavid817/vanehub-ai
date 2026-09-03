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

/** Where one span's bar sits, or why it has none. */
export type SpanBarPlacement =
  | { kind: "placed"; leftPx: number; widthPx: number; openEnded: boolean }
  /** The span carries no offset, so nothing here can say where it belongs. */
  | { kind: "unplaceable" };

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
 * `openEnded` is the running case: the bar starts where the span started and is drawn as
 * continuing rather than ending, because where it ends has not happened yet. A view renders that
 * differently — a fade, a stripe — but never as a bar with a right edge, which would be a claim.
 */
export function placeSpanBar(
  span: ExecutionSpanSummary,
  scale: TraceTimeScale,
): SpanBarPlacement {
  const offset = span.startOffsetMs;
  if (offset === undefined) return { kind: "unplaceable" };

  const pxPerMs = scale.contentWidthPx / scale.totalMs;
  const leftPx = offset * pxPerMs;
  const duration = span.completedDurationMs;
  if (duration === undefined) {
    // Runs to the end of the axis, and says so. The width is a placeholder for "still going", not
    // a measurement — which is why the flag travels with it rather than being inferred from the
    // number by whoever renders it.
    return {
      kind: "placed",
      leftPx,
      widthPx: Math.max(MIN_BAR_WIDTH_PX, scale.contentWidthPx - leftPx),
      openEnded: true,
    };
  }
  return {
    kind: "placed",
    leftPx,
    widthPx: Math.max(MIN_BAR_WIDTH_PX, duration * pxPerMs),
    openEnded: false,
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
