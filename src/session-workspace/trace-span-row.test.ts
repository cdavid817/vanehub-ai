import { describe, expect, it } from "vitest";
import type { ExecutionSpanSummary } from "../types/execution-observability";
import { spanAccessibleLabel } from "./trace-span-row";

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

/** Returns the key so an assertion can name the sentence a reader would actually hear. */
const t = (key: string, values?: Record<string, string | number>) =>
  values ? `${key}:${JSON.stringify(values)}` : key;

/**
 * The label is the whole chart for anyone who cannot see it, so every distinction the bar makes
 * visually has to survive into words. The one this guards is the distinction the bar itself was
 * getting wrong: an absent duration means "still running" only when the span has not ended.
 */
describe("span accessible label", () => {
  it("says a running span is still running", () => {
    const label = spanAccessibleLabel(span({ spanId: "a", status: "running" }), t);

    expect(label).toContain("traces.stillRunning");
  });

  it("does not call a terminated span with an unmeasurable duration still running", () => {
    const label = spanAccessibleLabel(
      span({ spanId: "a", status: "failed", endedAt: "2026-08-25T10:00:02.000Z" }),
      t,
    );

    // The span failed. Announcing it as running tells a screen-reader user the opposite of what
    // happened, and there is no colour or bar shape for them to check it against.
    expect(label).not.toContain("traces.stillRunning");
    expect(label).toContain("traces.durationUnknown");
  });

  it("does not call an incomplete span still running", () => {
    const label = spanAccessibleLabel(span({ spanId: "a", status: "incomplete" }), t);

    expect(label).not.toContain("traces.stillRunning");
    expect(label).toContain("traces.durationUnknown");
  });

  it("reports a measured duration as a measurement", () => {
    const label = spanAccessibleLabel(
      span({ spanId: "a", status: "succeeded", completedDurationMs: 250 }),
      t,
    );

    expect(label).toContain("traces.duration");
    expect(label).not.toContain("traces.stillRunning");
    expect(label).not.toContain("traces.durationUnknown");
  });

  it("still reports an unplaceable span separately from its duration", () => {
    const label = spanAccessibleLabel(
      span({ spanId: "a", status: "failed", startOffsetMs: undefined }),
      t,
    );

    // Position and duration are different absences: one says where it sat, the other how long it
    // took. A span can be missing either without the other being in doubt.
    expect(label).toContain("traces.unplaceable");
    expect(label).toContain("traces.durationUnknown");
  });
});
