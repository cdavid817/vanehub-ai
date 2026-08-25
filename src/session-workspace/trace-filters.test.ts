import { describe, expect, it } from "vitest";
import type { ExecutionSpanSummary } from "../types/execution-observability";
import {
  filterTraceSpans,
  hasActiveTraceFilters,
  matchesTraceFilters,
  NO_TRACE_FILTERS,
} from "./trace-filters";

function span(overrides: Partial<ExecutionSpanSummary> & { spanId: string }): ExecutionSpanSummary {
  return {
    parentSpanId: null,
    name: `span-${overrides.spanId}`,
    kind: "unknown",
    status: "succeeded",
    fidelity: "native",
    startedAt: "2026-08-25T10:00:00.000Z",
    endedAt: "2026-08-25T10:00:01.000Z",
    durationMs: 1000,
    errorClassification: null,
    attributes: {},
    depth: 0,
    startOffsetMs: 0,
    completedDurationMs: 1000,
    delegated: false,
    criticalPath: false,
    links: [],
    ...overrides,
  };
}

/**
 * Every filter here narrows what a reader is looking at, and narrowing has one cost that never
 * changes: a hidden span is indistinguishable from a span that never happened. So the tests are as
 * much about what the filters *report* as about what they keep.
 */
describe("trace filters", () => {
  it("keeps everything when nothing is filtering", () => {
    const spans = [span({ spanId: "a" }), span({ spanId: "b" })];

    const result = filterTraceSpans(spans, NO_TRACE_FILTERS);

    expect(result.spans).toHaveLength(2);
    expect(result.hiddenCount).toBe(0);
    expect(hasActiveTraceFilters(NO_TRACE_FILTERS)).toBe(false);
  });

  it("counts what it hid", () => {
    const spans = [
      span({ spanId: "a", status: "failed" }),
      span({ spanId: "b" }),
      span({ spanId: "c" }),
    ];

    const result = filterTraceSpans(spans, { ...NO_TRACE_FILTERS, failed: true });

    expect(result.spans.map((item) => item.spanId)).toEqual(["a"]);
    // One row on screen looks the same whether the run had one span or three. The count is what
    // stops a narrowed view from reading as a run that did almost nothing.
    expect(result.hiddenCount).toBe(2);
  });

  it("treats an unfinished span as something a failure hunt must see", () => {
    const spans = [span({ spanId: "a", status: "incomplete" })];

    const result = filterTraceSpans(spans, { ...NO_TRACE_FILTERS, failed: true });

    // A span whose end was never observed may have failed. Sorting it quietly into the successes
    // is how somebody scanning for failures misses the one that mattered.
    expect(result.spans).toHaveLength(1);
  });

  it("counts a span nobody counted an attempt for as not retried", () => {
    const untried = span({ spanId: "a" });
    const retried = span({ spanId: "b", attempt: 2 });
    // A producer that recorded attempt 1 said so; that is still not a retry.
    const firstAttempt = span({ spanId: "c", attempt: 1 });

    const result = filterTraceSpans([untried, retried, firstAttempt], {
      ...NO_TRACE_FILTERS,
      retried: true,
    });

    expect(result.spans.map((item) => item.spanId)).toEqual(["b"]);
  });

  it("narrows to the critical path and to delegation independently", () => {
    const spans = [
      span({ spanId: "critical", criticalPath: true }),
      span({ spanId: "delegated", delegated: true }),
    ];

    expect(
      filterTraceSpans(spans, { ...NO_TRACE_FILTERS, criticalPath: true }).spans.map((s) => s.spanId),
    ).toEqual(["critical"]);
    expect(
      filterTraceSpans(spans, { ...NO_TRACE_FILTERS, delegated: true }).spans.map((s) => s.spanId),
    ).toEqual(["delegated"]);
  });

  it("treats an opaque span and an unfinished one as the same kind of gap", () => {
    const spans = [
      span({ spanId: "opaque", fidelity: "opaque" }),
      span({ spanId: "unfinished", status: "incomplete" }),
      span({ spanId: "clean" }),
    ];

    const result = filterTraceSpans(spans, { ...NO_TRACE_FILTERS, gap: true });

    // Two shapes of the same problem: the runtime could not see inside, or saw the start and never
    // the end. Somebody auditing what a trace can be trusted about wants them together.
    expect(result.spans.map((item) => item.spanId)).toEqual(["opaque", "unfinished"]);
  });

  it("narrows by fidelity, and an empty fidelity set means every fidelity", () => {
    const spans = [
      span({ spanId: "native" }),
      span({ spanId: "inferred", fidelity: "inferred" }),
    ];

    expect(
      filterTraceSpans(spans, { ...NO_TRACE_FILTERS, fidelities: ["inferred"] }).spans.map((s) => s.spanId),
    ).toEqual(["inferred"]);
    expect(filterTraceSpans(spans, { ...NO_TRACE_FILTERS, fidelities: [] }).spans).toHaveLength(2);
  });

  it("combines active filters conjunctively", () => {
    const both = span({ spanId: "both", status: "failed", criticalPath: true });
    const onlyFailed = span({ spanId: "failed", status: "failed" });
    const onlyCritical = span({ spanId: "critical", criticalPath: true });

    const result = filterTraceSpans([both, onlyFailed, onlyCritical], {
      ...NO_TRACE_FILTERS,
      failed: true,
      criticalPath: true,
    });

    // "Failed and on the critical path" is the question somebody debugging a slow failure has. A
    // union would answer a different one while looking like it had answered theirs.
    expect(result.spans.map((item) => item.spanId)).toEqual(["both"]);
  });

  it("can hide everything, and says so through the count", () => {
    const spans = [span({ spanId: "a" }), span({ spanId: "b" })];

    const result = filterTraceSpans(spans, { ...NO_TRACE_FILTERS, failed: true });

    expect(result.spans).toHaveLength(0);
    // An empty waterfall with a nonzero hidden count is a different fact from an empty one with a
    // zero count, and the view renders them with different words.
    expect(result.hiddenCount).toBe(2);
  });

  it("matches one span the same way the list does", () => {
    const failed = span({ spanId: "a", status: "failed" });

    expect(matchesTraceFilters(failed, { ...NO_TRACE_FILTERS, failed: true })).toBe(true);
    expect(matchesTraceFilters(failed, { ...NO_TRACE_FILTERS, delegated: true })).toBe(false);
  });
});
