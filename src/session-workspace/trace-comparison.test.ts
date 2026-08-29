import { describe, expect, it } from "vitest";
import type {
  ExecutionSpanSummary,
  ExecutionTimeline,
} from "../types/execution-observability";
import { comparedDelta, compareRuns } from "./trace-comparison";

function span(overrides: Partial<ExecutionSpanSummary> & { spanId: string }): ExecutionSpanSummary {
  return {
    parentSpanId: null,
    name: `span-${overrides.spanId}`,
    kind: "tool",
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

function timeline(
  spans: ExecutionSpanSummary[],
  run: Partial<ExecutionTimeline["run"]> = {},
): ExecutionTimeline {
  return {
    run: {
      runId: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
      traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
      rootSpanId: "00f067aa0ba902b7",
      source: "desktop",
      sourceId: null,
      status: "succeeded",
      startedAt: "2026-08-25T10:00:00.000Z",
      endedAt: "2026-08-25T10:00:03.000Z",
      durationMs: 3000,
      sessionId: "session-1",
      operationId: null,
      agentId: "claude-code",
      ...run,
    },
    spans,
    events: [],
  };
}

/**
 * Two runs compared by what was counted, never by what was said.
 *
 * The failure this guards against is specific and quiet: a run whose spans are half opaque reports
 * fewer tool calls than it made, and set beside a fully observed run it looks like it did less
 * work. A reader draws a conclusion about behaviour from a difference in observation.
 */
describe("run comparison", () => {
  it("compares statuses directly", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" })], { status: "succeeded" }),
      timeline([span({ spanId: "b" })], { status: "failed" }),
    );

    expect(comparison.status).toMatchObject({
      left: "succeeded",
      right: "failed",
      same: false,
    });
  });

  it("refuses to compare durations while either run is going", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" })], { durationMs: 3000 }),
      timeline([span({ spanId: "b" })], { durationMs: null, endedAt: null, status: "running" }),
    );

    // Comparing elapsed-so-far to a finished total would report the running one as faster, which
    // is not a fact about either.
    expect(comparison.duration.comparable).toBe(false);
    expect(comparedDelta(comparison.duration)).toBeNull();
  });

  it("compares durations once both have one", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" })], { durationMs: 3000 }),
      timeline([span({ spanId: "b" })], { durationMs: 5000 }),
    );

    expect(comparedDelta(comparison.duration)).toBe(2000);
  });

  it("marks counts as floors when a run has an opaque span", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" }), span({ spanId: "b" })]),
      timeline([span({ spanId: "c", fidelity: "opaque" })]),
    );

    // "2 vs 1" would say the second run did less. What it should say is that the second run was
    // watched less closely, and anything inside that opaque span is uncounted.
    expect(comparison.spans.comparable).toBe(false);
    expect(comparedDelta(comparison.spans)).toBeNull();
  });

  it("marks counts as floors when a span's end was never observed", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" })]),
      timeline([span({ spanId: "b", status: "incomplete" })]),
    );

    // Anything after the point observation stopped is uncounted, the same way it is for an
    // opaque span.
    expect(comparison.spans.comparable).toBe(false);
  });

  it("compares freely when both runs were fully observed", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" }), span({ spanId: "b" })]),
      timeline([span({ spanId: "c" })]),
    );

    expect(comparison.spans.comparable).toBe(true);
    expect(comparedDelta(comparison.spans)).toBe(-1);
  });

  it("says so when only one run has an observation gap", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" })]),
      timeline([span({ spanId: "b", fidelity: "opaque" })]),
    );

    // Stated once, above everything it qualifies. A reader who misses it reads the whole
    // comparison as a set of measurements.
    expect(comparison.observationDiffers).toBe(true);
  });

  it("does not claim a difference in observation when both runs have one", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a", fidelity: "opaque" })]),
      timeline([span({ spanId: "b", fidelity: "opaque" })]),
    );

    expect(comparison.observationDiffers).toBe(false);
    // Both are still floors, so their counts remain incomparable.
    expect(comparison.spans.comparable).toBe(false);
  });

  it("counts failures including spans whose end was never seen", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a", status: "failed" })]),
      timeline([
        span({ spanId: "b", status: "cancelled" }),
        span({ spanId: "c", status: "incomplete" }),
      ]),
    );

    expect(comparison.failures.left).toBe(1);
    expect(comparison.failures.right).toBe(2);
  });

  it("counts spans by kind and leaves unclassified ones out", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a", kind: "tool" }), span({ spanId: "b", kind: "unknown" })]),
      timeline([span({ spanId: "c", kind: "tool" }), span({ spanId: "d", kind: "mcp" })]),
    );

    const kinds = comparison.toolCounts.map((entry) => entry.kind);
    // `unknown` is not a kind of work, it is the absence of a claim about one. Comparing it as a
    // category would invite a reader to draw a conclusion from how much nobody classified.
    expect(kinds).not.toContain("unknown");
    expect(comparison.toolCounts.find((entry) => entry.kind === "tool")).toMatchObject({
      left: 1,
      right: 1,
    });
    expect(comparison.toolCounts.find((entry) => entry.kind === "mcp")).toMatchObject({
      left: 0,
      right: 1,
    });
  });

  it("counts file-touching spans without naming a single file", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a", kind: "file" })]),
      timeline([
        span({
          spanId: "b",
          links: [
            { runId: "run-1", traceId: "trace-1", spanId: "f", relationship: "file-change" },
          ],
        }),
      ]),
    );

    // Which files a run touched is a path, and two runs' paths side by side is a content
    // comparison wearing a different name.
    expect(comparison.changes.left).toBe(1);
    expect(comparison.changes.right).toBe(1);
    expect(JSON.stringify(comparison)).not.toContain("file-change");
  });

  it("reports the fidelity mix of each run", () => {
    const comparison = compareRuns(
      timeline([span({ spanId: "a" }), span({ spanId: "b", fidelity: "inferred" })]),
      timeline([span({ spanId: "c", fidelity: "opaque" })]),
    );

    expect(comparison.usageQuality.left).toMatchObject({ native: 1, inferred: 1, opaque: 0 });
    expect(comparison.usageQuality.right).toMatchObject({ native: 0, opaque: 1 });
  });

  it("carries no span name, message or attribute anywhere in its result", () => {
    const comparison = compareRuns(
      timeline([
        span({
          spanId: "a",
          name: "secret-looking-span-name",
          attributes: { "gen_ai.request.model": "some-model" },
        }),
      ]),
      timeline([span({ spanId: "b" })]),
    );

    // The whole point of comparing counts is that nothing textual crosses into the comparison.
    const serialised = JSON.stringify(comparison);
    expect(serialised).not.toContain("secret-looking-span-name");
    expect(serialised).not.toContain("some-model");
  });

  it("compares two empty runs without inventing a difference", () => {
    const comparison = compareRuns(timeline([]), timeline([]));

    expect(comparison.spans).toMatchObject({ left: 0, right: 0, comparable: true });
    expect(comparedDelta(comparison.spans)).toBe(0);
    expect(comparison.toolCounts).toEqual([]);
  });
});
