import { describe, expect, it } from "vitest";
import type { ExecutionSpanSummary } from "../types/execution-observability";
import {
  flattenSpanRows,
  MIN_BAR_WIDTH_PX,
  MAX_ZOOM,
  MIN_ZOOM,
  placeSpanBar,
  traceAxisTicks,
  traceTimeScale,
} from "./trace-time-scale";

function span(overrides: Partial<ExecutionSpanSummary> & { spanId: string }): ExecutionSpanSummary {
  return {
    parentSpanId: null,
    name: `span-${overrides.spanId}`,
    kind: "unknown",
    status: "succeeded",
    fidelity: "native",
    startedAt: "2026-08-25T10:00:00.000Z",
    endedAt: null,
    durationMs: null,
    errorClassification: null,
    attributes: {},
    depth: 0,
    delegated: false,
    criticalPath: false,
    links: [],
    ...overrides,
  };
}

/**
 * The interesting cases are the ones where a bar must *not* be drawn in the obvious place.
 *
 * Both failures this guards against are one `?? 0` away and neither is visible from a screenshot:
 * an unplaceable span rendered at the origin puts work at the start of the run that did not happen
 * there, and a running span drawn to a right edge shows a measurement nobody made.
 */
describe("trace time scale", () => {
  it("spans the axis from the furthest observed end rather than a run duration", () => {
    const scale = traceTimeScale(
      [
        span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 1000 }),
        span({ spanId: "b", startOffsetMs: 500, completedDurationMs: 2000 }),
      ],
      800,
      1,
    );

    expect(scale.totalMs).toBe(2500);
  });

  it("lets a running span contribute only where it started", () => {
    const scale = traceTimeScale(
      [
        span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 400 }),
        // Still going. Assuming a length would put the axis end somewhere nothing reached.
        span({ spanId: "b", startOffsetMs: 900 }),
      ],
      800,
      1,
    );

    expect(scale.totalMs).toBe(900);
  });

  it("never collapses the axis to zero", () => {
    // The first frame of a run legitimately has one span at offset zero with no duration yet, and
    // every division against a zero-length axis is undefined.
    const scale = traceTimeScale([span({ spanId: "a", startOffsetMs: 0 })], 800, 1);

    expect(scale.totalMs).toBeGreaterThan(0);
    expect(Number.isFinite(scale.contentWidthPx)).toBe(true);
  });

  it("widens the content rather than the axis when zooming", () => {
    const spans = [span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 1000 })];
    const fitted = traceTimeScale(spans, 800, 1);
    const zoomed = traceTimeScale(spans, 800, 4);

    // Zoom is horizontal scrolling, not a different time range: the same milliseconds occupy more
    // pixels, so a reader who zoomed in has not been shown a different run.
    expect(zoomed.totalMs).toBe(fitted.totalMs);
    expect(zoomed.contentWidthPx).toBe(fitted.contentWidthPx * 4);
  });

  it("clamps the zoom to its bounds", () => {
    const spans = [span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 10 })];

    expect(traceTimeScale(spans, 800, 0.1).zoom).toBe(MIN_ZOOM);
    expect(traceTimeScale(spans, 800, 10_000).zoom).toBe(MAX_ZOOM);
  });

  it("refuses to place a span that carries no offset", () => {
    const scale = traceTimeScale([span({ spanId: "a", startOffsetMs: 0 })], 800, 1);

    const placement = placeSpanBar(span({ spanId: "b" }), scale);

    // Not `leftPx: 0`. A bar at the origin is a definite claim about when the work happened, and
    // this span is precisely the one nothing can say that about.
    expect(placement.kind).toBe("unplaceable");
  });

  it("draws a running span open-ended rather than to a right edge", () => {
    const scale = traceTimeScale(
      [span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 1000 })],
      800,
      1,
    );

    const placement = placeSpanBar(span({ spanId: "b", startOffsetMs: 500 }), scale);

    expect(placement).toMatchObject({ kind: "placed", openEnded: true });
    // The flag travels with the placement rather than being inferred from the width, so a renderer
    // cannot accidentally draw a definite end on a span that has none.
    if (placement.kind === "placed") expect(placement.widthPx).toBeGreaterThan(0);
  });

  it("marks a finished span as having a definite end", () => {
    const scale = traceTimeScale(
      [span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 1000 })],
      1000,
      1,
    );

    const placement = placeSpanBar(
      span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 500 }),
      scale,
    );

    expect(placement).toMatchObject({ kind: "placed", openEnded: false, leftPx: 0 });
    if (placement.kind === "placed") expect(placement.widthPx).toBe(500);
  });

  it("keeps a very short span visible", () => {
    const scale = traceTimeScale(
      [span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 100_000 })],
      800,
      1,
    );

    const placement = placeSpanBar(
      span({ spanId: "b", startOffsetMs: 0, completedDurationMs: 1 }),
      scale,
    );

    // An invisible bar reads as "it did not happen" rather than "it was fast".
    if (placement.kind === "placed") {
      expect(placement.widthPx).toBeGreaterThanOrEqual(MIN_BAR_WIDTH_PX);
    }
  });

  it("bounds the number of axis ticks however far the reader zooms", () => {
    const spans = [span({ spanId: "a", startOffsetMs: 0, completedDurationMs: 5000 })];

    const ticks = traceAxisTicks(traceTimeScale(spans, 800, MAX_ZOOM));

    // Past a certain density the labels overlap into a band, and a reader cannot tell an axis with
    // too many ticks from one that is broken.
    expect(ticks.length).toBeLessThanOrEqual(8);
    expect(ticks[0]).toBe(0);
    expect(ticks.at(-1)).toBe(5000);
  });
});

describe("span row flattening", () => {
  it("walks depth first, so a span is followed by what it caused", () => {
    const rows = flattenSpanRows([
      span({ spanId: "root" }),
      span({ spanId: "child", parentSpanId: "root" }),
      span({ spanId: "grandchild", parentSpanId: "child" }),
      span({ spanId: "sibling", parentSpanId: "root" }),
    ]);

    expect(rows.map((row) => row.span.spanId)).toEqual([
      "root",
      "child",
      "grandchild",
      "sibling",
    ]);
    expect(rows.map((row) => row.depth)).toEqual([0, 1, 2, 1]);
  });

  it("treats a span whose parent is absent as a root rather than hiding it", () => {
    const rows = flattenSpanRows([span({ spanId: "orphan", parentSpanId: "never-recorded" })]);

    // Its parent was filtered out or never written. Dropping it would remove real work from the
    // list because of something that happened to a different span.
    expect(rows).toHaveLength(1);
    expect(rows[0].depth).toBe(0);
  });

  it("lists every span exactly once even when the parent chain loops", () => {
    const rows = flattenSpanRows([
      span({ spanId: "a", parentSpanId: "b" }),
      span({ spanId: "b", parentSpanId: "a" }),
    ]);

    const ids = rows.map((row) => row.span.spanId).sort();
    expect(ids).toEqual(["a", "b"]);
    // A cycle is a producer bug that arrives as data. It must not become an infinite list, and it
    // must not become a shorter one either.
    expect(rows).toHaveLength(2);
  });

  it("ignores a span that claims to be its own parent", () => {
    const rows = flattenSpanRows([span({ spanId: "self", parentSpanId: "self" })]);

    expect(rows).toHaveLength(1);
    expect(rows[0].depth).toBe(0);
  });

  it("returns nothing for an empty trace", () => {
    expect(flattenSpanRows([])).toEqual([]);
  });
});

describe("span topology carried over from the retired tree builder", () => {
  it("walks a three-level chain and keeps an orphan visible beside it", () => {
    const rows = flattenSpanRows([
      span({ spanId: "root" }),
      span({ spanId: "child", parentSpanId: "root" }),
      span({ spanId: "grandchild", parentSpanId: "child" }),
      span({ spanId: "opaque-gap", parentSpanId: "missing" }),
    ]);

    // The old tree builder returned roots; the flattener returns rows with depths. The property
    // is the same one: nothing is hidden because of a parent that was never recorded.
    expect(rows.map((row) => [row.span.spanId, row.depth])).toEqual([
      ["root", 0],
      ["child", 1],
      ["grandchild", 2],
      ["opaque-gap", 0],
    ]);
  });
});
