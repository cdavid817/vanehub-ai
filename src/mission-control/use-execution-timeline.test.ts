// @vitest-environment jsdom

import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary, ExecutionTimeline } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import { useExecutionTimeline } from "./use-execution-timeline";

afterEach(() => vi.restoreAllMocks());

function run(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:00:00.000Z",
    endedAt: "2026-08-16T00:10:00.000Z", projectId: null, workspace: null, phase: null, attention: null,
    reasonCode: null, verification: "unavailable", tokens: null, cost: null, actions: [],
    navigation: { kind: "session", id: "session-1", sessionId: null },
    runner: null,
    ...overrides,
  };
}

function execRun(overrides: Partial<ExecutionRunSummary> = {}): ExecutionRunSummary {
  return {
    runId: "exec-1", traceId: "trace-1", rootSpanId: "span-1", source: "desktop", status: "succeeded",
    startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:10:00.000Z", sessionId: "session-1",
    ...overrides,
  };
}

function timeline(overrides: Partial<ExecutionTimeline> = {}): ExecutionTimeline {
  return { run: execRun(), spans: [], events: [], ...overrides };
}

// A matching execution run for `run()` above, resolvable by `resolveSessionExecutionContext`.
function mockMatchingListRuns() {
  return vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
    items: [execRun({ startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:20:00.000Z" })],
    nextPageToken: null,
  });
}

describe("useExecutionTimeline", () => {
  it("starts in the loading state before the resolver settles", () => {
    mockMatchingListRuns();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline());
    const target = run();

    const { result } = renderHook(() => useExecutionTimeline(target));

    expect(result.current.status).toBe("loading");
  });

  it("fetches the timeline by the resolved execution run's own runId, not the Mission Control run's id", async () => {
    mockMatchingListRuns();
    const getTimelineSpy = vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline());
    const target = run();

    const { result } = renderHook(() => useExecutionTimeline(target));

    await waitFor(() => expect(result.current.status).toBe("ready"));
    expect(getTimelineSpy).toHaveBeenCalledWith("exec-1");
    expect(getTimelineSpy).not.toHaveBeenCalledWith("run-1");
  });

  it("resolves to empty, without fetching a timeline, when the run has no session link", async () => {
    const getTimelineSpy = vi.spyOn(executionObservabilityService, "getTimeline");
    const target = run({ navigation: null });

    const { result } = renderHook(() => useExecutionTimeline(target));

    await waitFor(() => expect(result.current.status).toBe("empty"));
    expect(getTimelineSpy).not.toHaveBeenCalled();
  });

  it("resolves to empty when no execution run overlaps the Mission Control run's window", async () => {
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2020-01-01T00:00:00.000Z", endedAt: "2020-01-01T00:05:00.000Z" })],
      nextPageToken: null,
    });
    const target = run();

    const { result } = renderHook(() => useExecutionTimeline(target));

    await waitFor(() => expect(result.current.status).toBe("empty"));
  });

  it("resolves to error when the timeline fetch rejects", async () => {
    mockMatchingListRuns();
    vi.spyOn(executionObservabilityService, "getTimeline").mockRejectedValue(new Error("token=secret"));
    const target = run();

    const { result } = renderHook(() => useExecutionTimeline(target));

    await waitFor(() => expect(result.current.status).toBe("error"));
  });

  it("resolves to error when the resolver's own listRuns call rejects", async () => {
    vi.spyOn(executionObservabilityService, "listRuns").mockRejectedValue(new Error("boom"));
    const target = run();

    const { result } = renderHook(() => useExecutionTimeline(target));

    await waitFor(() => expect(result.current.status).toBe("error"));
  });

  it("keeps a ready timeline with zero spans distinct from the empty state", async () => {
    mockMatchingListRuns();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline({ spans: [] }));
    const target = run();

    const { result } = renderHook(() => useExecutionTimeline(target));

    // A resolved run whose timeline genuinely has no spans is real, valid data — "ready" with an
    // empty list — not the same situation as the resolver finding nothing to join at all ("empty").
    await waitFor(() => expect(result.current.status).toBe("ready"));
    expect(result.current.status).not.toBe("empty");
    if (result.current.status === "ready") expect(result.current.timeline.spans).toHaveLength(0);
  });
});
