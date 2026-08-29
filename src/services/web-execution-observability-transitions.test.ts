import { beforeEach, describe, expect, it } from "vitest";
import {
  completeRunningExecutionForTest,
  resetWebExecutionObservabilityForTest,
  webExecutionObservabilityClient,
} from "./web-execution-observability-client";

const RUNNING_RUN = "018f0f17-4d6a-7e20-b41d-66c5271a28d2";

/**
 * The browser build has to be able to cross the running-to-terminal boundary.
 *
 * Several rules in this design only hold on one side of it — a running span has no duration, and a
 * run with anything still running has no critical path — and a fixture permanently on one side
 * makes them unobservable. A mock that only ever served finished traces would be the one runtime
 * where "we do not know yet" is never rendered, which is precisely the state the waterfall exists
 * to show honestly.
 */
describe("web execution observability transitions", () => {
  beforeEach(() => {
    resetWebExecutionObservabilityForTest();
  });

  it("serves a run that is still going", async () => {
    const timeline = await webExecutionObservabilityClient.getTimeline(RUNNING_RUN);

    expect(timeline.run.status).toBe("running");
    expect(timeline.run.endedAt).toBeNull();
    expect(timeline.run.durationMs).toBeNull();
  });

  it("gives a running span no completed duration", async () => {
    const timeline = await webExecutionObservabilityClient.getTimeline(RUNNING_RUN);

    for (const span of timeline.spans) {
      // Elapsed-so-far would make a running span look like one that finished in exactly that time.
      expect(span.completedDurationMs).toBeUndefined();
      expect(span.endedAt).toBeNull();
    }
  });

  it("puts no span on the critical path while the run is unfinished", async () => {
    const timeline = await webExecutionObservabilityClient.getTimeline(RUNNING_RUN);

    // A critical path says which work the duration depended on, and the unfinished span may yet
    // become the longest.
    expect(timeline.spans.every((span) => !span.criticalPath)).toBe(true);
  });

  it("still places every running span on the time axis", async () => {
    const timeline = await webExecutionObservabilityClient.getTimeline(RUNNING_RUN);

    // Where a span started is known even while it runs, so a bar can be positioned. Only its
    // length is unknown, and that is the field that stays absent.
    for (const span of timeline.spans) {
      expect(typeof span.startOffsetMs).toBe("number");
    }
  });

  it("completes the run, and only then derives durations and a critical path", async () => {
    completeRunningExecutionForTest();

    const timeline = await webExecutionObservabilityClient.getTimeline(RUNNING_RUN);

    expect(timeline.run.status).toBe("succeeded");
    expect(timeline.run.durationMs).toBe(3000);
    for (const span of timeline.spans) {
      expect(span.completedDurationMs).toBeGreaterThan(0);
      expect(span.criticalPath).toBe(true);
    }
  });

  it("restores the running fixture, so the boundary can be crossed more than once", async () => {
    completeRunningExecutionForTest();
    resetWebExecutionObservabilityForTest();

    const timeline = await webExecutionObservabilityClient.getTimeline(RUNNING_RUN);

    expect(timeline.run.status).toBe("running");
    expect(timeline.spans.every((span) => span.completedDurationMs === undefined)).toBe(true);
  });

  it("classifies its fixture spans from attributes rather than names", async () => {
    const finished = await webExecutionObservabilityClient.getTimeline(
      "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
    );

    const byId = new Map(finished.spans.map((span) => [span.spanId, span]));
    // `execute_tool search` carries `gen_ai.tool.name`, and its name would also have matched the
    // old substring rule — so the interesting one is the MCP span, whose kind comes from
    // `rpc.system` rather than from the "mcp" in its name.
    expect(byId.get("b7ad6b7169203331")?.kind).toBe("tool");
    expect(byId.get("b7ad6b7169203333")?.kind).toBe("mcp");
    expect(byId.get("00f067aa0ba902b7")?.kind).toBe("model");
  });

  it("marks a delegated span as delegated and carries the attempt its producer counted", async () => {
    const running = await webExecutionObservabilityClient.getTimeline(RUNNING_RUN);

    const delegated = running.spans.find((span) => span.kind === "delegation");
    expect(delegated?.delegated).toBe(true);
    expect(delegated?.attempt).toBe(2);
    // The other span has no attempt, and that absence is the point: defaulting it to 1 would
    // assert a retry history nobody observed.
    expect(running.spans.find((span) => span.kind === "container")?.attempt).toBeUndefined();
  });
});
