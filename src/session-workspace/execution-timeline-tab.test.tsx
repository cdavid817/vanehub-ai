import { describe, expect, it } from "vitest";
import { spanTree } from "./execution-timeline-tab";
import type { ExecutionSpanSummary } from "../types/execution-observability";

const span = (spanId: string, parentSpanId: string | null): ExecutionSpanSummary => ({
  spanId,
  parentSpanId,
  name: `span-${spanId}`,
  status: "succeeded",
  fidelity: "native",
  startedAt: "2026-07-23T00:00:00Z",
  endedAt: "2026-07-23T00:00:01Z",
  durationMs: 1000,
  errorClassification: null,
  attributes: {},
});

describe("execution timeline topology", () => {
  it("builds parent/child topology and leaves missing parents as visible roots", () => {
    const roots = spanTree([
      span("root", null),
      span("child", "root"),
      span("grandchild", "child"),
      span("opaque-gap", "missing"),
    ]);
    expect(roots.map((node) => node.span.spanId)).toEqual(["root", "opaque-gap"]);
    expect(roots[0]?.children[0]?.children[0]?.span.spanId).toBe("grandchild");
  });
});
