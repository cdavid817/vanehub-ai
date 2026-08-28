import type {
  ExecutionSpanSummary,
  ExecutionTimeline,
} from "../types/execution-observability";

/**
 * The trace fixtures the browser build serves, and the transition it can be driven through.
 *
 * Every derived field is written out rather than computed, because these are fixtures: the values
 * are fixed, and a mock that recomputed them would be a second implementation of the native
 * derivation that could disagree with it while both looked right. What matters is that they follow
 * the same rules — a running span carries no `completedDurationMs`, and no span is on the critical
 * path until the whole run has finished.
 */

/** A run that is still going. Nothing in it may claim a duration or a critical path. */
const RUNNING_RUN_ID = "018f0f17-4d6a-7e20-b41d-66c5271a28d2";

function span(overrides: Partial<ExecutionSpanSummary> & Pick<ExecutionSpanSummary, "spanId" | "name" | "startedAt">): ExecutionSpanSummary {
  return {
    parentSpanId: null,
    kind: "unknown",
    status: "running",
    fidelity: "native",
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

export const executionTimelineFixtures: ExecutionTimeline[] = [
  {
    run: {
      runId: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
      traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
      rootSpanId: "00f067aa0ba902b7",
      source: "desktop",
      sourceId: null,
      status: "succeeded",
      startedAt: "2026-07-23T08:00:00.000Z",
      endedAt: "2026-07-23T08:00:02.400Z",
      durationMs: 2400,
      sessionId: "web-session-1",
      operationId: "web-operation-observability",
      agentId: "codex-cli",
    },
    spans: [
      span({
        spanId: "00f067aa0ba902b7",
        name: "vanehub.task.execute",
        kind: "model",
        status: "succeeded",
        startedAt: "2026-07-23T08:00:00.000Z",
        endedAt: "2026-07-23T08:00:02.400Z",
        durationMs: 2400,
        attributes: { "gen_ai.provider.name": "openai" },
        startOffsetMs: 0,
        completedDurationMs: 2400,
      }),
      span({
        spanId: "b7ad6b7169203331",
        parentSpanId: "00f067aa0ba902b7",
        name: "execute_tool search",
        kind: "tool",
        status: "incomplete",
        fidelity: "inferred",
        startedAt: "2026-07-23T08:00:01.000Z",
        endedAt: "2026-07-23T08:00:02.300Z",
        errorClassification: "missing_terminal_boundary",
        attributes: { "gen_ai.tool.name": "search" },
        depth: 1,
        startOffsetMs: 1000,
        completedDurationMs: 1300,
      }),
      span({
        spanId: "b7ad6b7169203333",
        parentSpanId: "00f067aa0ba902b7",
        name: "mcp.client request",
        kind: "mcp",
        status: "incomplete",
        fidelity: "opaque",
        startedAt: "2026-07-23T08:00:01.100Z",
        errorClassification: "traffic_not_managed",
        attributes: { "rpc.system": "mcp" },
        depth: 1,
        startOffsetMs: 1100,
        // No `completedDurationMs`: this span never ended, and the run therefore has no critical
        // path either — which is why none of these three carry one.
      }),
    ],
    events: [
      {
        sequence: 1,
        spanId: "00f067aa0ba902b7",
        name: "process.spawned",
        timestamp: "2026-07-23T08:00:00.500Z",
        attributes: { "process.pid.observed": true },
      },
    ],
  },
  {
    // The seed the pagination filler clones. Kept second because that loop copies `timelines[1]`,
    // and a run with spans would make every cloned page carry the same three of them.
    run: {
      runId: "018f0f17-4d6a-7e20-b41d-66c5271a28d1",
      traceId: "0af7651916cd43dd8448eb211c80319c",
      rootSpanId: "b7ad6b7169203332",
      source: "scheduled",
      sourceId: "web-schedule-1",
      status: "failed",
      startedAt: "2026-07-22T08:00:00.000Z",
      endedAt: "2026-07-22T08:00:00.100Z",
      durationMs: 100,
      sessionId: "web-session-1",
      operationId: "web-operation-scheduled",
      agentId: "gemini-cli",
    },
    spans: [],
    events: [],
  },
  {
    run: {
      runId: RUNNING_RUN_ID,
      traceId: "1af7651916cd43dd8448eb211c80319d",
      rootSpanId: "c7ad6b7169203340",
      source: "desktop",
      sourceId: null,
      status: "running",
      startedAt: "2026-07-24T08:00:00.000Z",
      endedAt: null,
      durationMs: null,
      sessionId: "web-session-1",
      operationId: "web-operation-running",
      agentId: "claude-code",
    },
    spans: [
      span({
        spanId: "c7ad6b7169203340",
        name: "vanehub.task.execute",
        kind: "container",
        startedAt: "2026-07-24T08:00:00.000Z",
        attributes: { "vanehub.span.kind": "container" },
        startOffsetMs: 0,
      }),
      span({
        spanId: "c7ad6b7169203341",
        parentSpanId: "c7ad6b7169203340",
        name: "delegate research",
        kind: "delegation",
        startedAt: "2026-07-24T08:00:00.400Z",
        attributes: { "vanehub.delegation.target": "researcher" },
        depth: 1,
        startOffsetMs: 400,
        delegated: true,
        attempt: 2,
      }),
    ],
    events: [],
  },
];

/**
 * Advances the running fixture to a terminal state.
 *
 * Deterministic by construction: fixed timestamps, fixed durations, and the critical path becomes
 * derivable at exactly the moment the last span ends — which is the transition worth being able to
 * drive, because the "no critical path while anything runs" rule is only observable by crossing it.
 */
export function completeRunningTimelineFixture(timelines: ExecutionTimeline[]): void {
  const running = timelines.find((timeline) => timeline.run.runId === RUNNING_RUN_ID);
  if (!running || running.run.status !== "running") return;

  running.run.status = "succeeded";
  running.run.endedAt = "2026-07-24T08:00:03.000Z";
  running.run.durationMs = 3000;
  running.spans = running.spans.map((item) => {
    const endedAt = item.spanId === "c7ad6b7169203340"
      ? "2026-07-24T08:00:03.000Z"
      : "2026-07-24T08:00:02.900Z";
    const startOffsetMs = item.startOffsetMs ?? 0;
    return {
      ...item,
      status: "succeeded",
      endedAt,
      durationMs: 3000 - startOffsetMs,
      completedDurationMs: 3000 - startOffsetMs,
      // Both are on the path now: the child ended last and the parent is its ancestor. Neither
      // could be marked before, because the run had not finished.
      criticalPath: true,
    };
  });
}

/**
 * Enough runs to page through, cloned from the scheduled fixture.
 *
 * Built here rather than in the client because the client rebuilds from these on reset — a filler
 * added outside would vanish the first time a test reset the adapter, and the pagination
 * assertions would then fail for a reason that has nothing to do with pagination.
 */
function paginationFiller(seed: ExecutionTimeline): ExecutionTimeline[] {
  return Array.from({ length: 19 }, (_unused, offset) => {
    const index = offset + 2;
    const suffix = index.toString(16).padStart(12, "0");
    return {
      run: {
        ...seed.run,
        runId: `018f0f17-4d6a-7e20-b41d-${suffix}`,
        startedAt: `2026-07-${String(22 - (index % 10)).padStart(2, "0")}T08:00:00.000Z`,
        operationId: `web-operation-${index}`,
      },
      spans: [],
      events: [],
    };
  });
}

/** Restores the running fixture, so a test can cross the transition more than once. */
export function resetExecutionTimelineFixtures(): ExecutionTimeline[] {
  const base = executionTimelineFixtures.map((timeline) => ({
    run: { ...timeline.run },
    spans: timeline.spans.map((item) => ({ ...item })),
    events: timeline.events.map((item) => ({ ...item })),
  }));
  return [...base, ...paginationFiller(base[1]!)];
}
