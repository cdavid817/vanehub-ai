// @vitest-environment jsdom

import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary, ExecutionTimeline } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import { useExecutionTimeline } from "./use-execution-timeline";

afterEach(() => vi.restoreAllMocks());

// Distinct from any real missionControl.*.empty/.error copy on purpose, so a passing assertion
// proves the hook actually plumbed the caller's own strings through rather than happening to match
// a hardcoded default.
const UNAVAILABLE_MESSAGE = "test-unavailable-message";
const ERROR_MESSAGE = "test-error-message";

function renderTimeline(target: MissionControlRunSummary) {
  return renderHook(() => useExecutionTimeline(target, UNAVAILABLE_MESSAGE, ERROR_MESSAGE));
}

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
  it("starts loading with no data before the resolver settles", async () => {
    await activateAppLanguage("en");
    mockMatchingListRuns();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline());
    const target = run();

    const { result } = renderTimeline(target);

    expect(result.current.initialLoading).toBe(true);
    expect(result.current.data).toBeUndefined();
  });

  it("fetches the timeline by the resolved execution run's own runId, not the Mission Control run's id", async () => {
    await activateAppLanguage("en");
    mockMatchingListRuns();
    const getTimelineSpy = vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline());
    const target = run();

    const { result } = renderTimeline(target);

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(getTimelineSpy).toHaveBeenCalledWith("exec-1");
    expect(getTimelineSpy).not.toHaveBeenCalledWith("run-1");
    expect(result.current.error).toBeUndefined();
  });

  it("resolves to an unavailable state, without fetching a timeline, when the run has no session link", async () => {
    await activateAppLanguage("en");
    const getTimelineSpy = vi.spyOn(executionObservabilityService, "getTimeline");
    const target = run({ navigation: null });

    const { result } = renderTimeline(target);

    await waitFor(() => expect(result.current.initialLoading).toBe(false));
    expect(result.current.error?.kind).toBe("unavailable");
    // Proves the caller's own message reaches the state, not a hardcoded default this hook picked.
    expect(result.current.error?.message).toBe(UNAVAILABLE_MESSAGE);
    expect(result.current.data).toBeUndefined();
    expect(getTimelineSpy).not.toHaveBeenCalled();
  });

  it("resolves to an unavailable state when no execution run overlaps the Mission Control run's window", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2020-01-01T00:00:00.000Z", endedAt: "2020-01-01T00:05:00.000Z" })],
      nextPageToken: null,
    });
    const target = run();

    const { result } = renderTimeline(target);

    await waitFor(() => expect(result.current.error?.kind).toBe("unavailable"));
  });

  it("resolves to a retryable error, without leaking the raw reason, when the timeline fetch rejects", async () => {
    await activateAppLanguage("en");
    mockMatchingListRuns();
    vi.spyOn(executionObservabilityService, "getTimeline").mockRejectedValue(new Error("token=secret"));
    const target = run();

    const { result } = renderTimeline(target);

    await waitFor(() => expect(result.current.error?.kind).toBe("error"));
    expect(result.current.error?.retryable).toBe(true);
    expect(result.current.error?.message).not.toContain("secret");
    expect(result.current.error?.message).toBe(ERROR_MESSAGE);
  });

  it("resolves to an error when the resolver's own listRuns call rejects", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockRejectedValue(new Error("boom"));
    const target = run();

    const { result } = renderTimeline(target);

    await waitFor(() => expect(result.current.error?.kind).toBe("error"));
  });

  it("keeps a ready timeline with zero spans distinct from the unavailable state", async () => {
    await activateAppLanguage("en");
    mockMatchingListRuns();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline({ spans: [] }));
    const target = run();

    const { result } = renderTimeline(target);

    // A resolved run whose timeline genuinely has no spans is real, valid data, not the same
    // situation as the resolver finding nothing to join at all.
    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.error).toBeUndefined();
    expect(result.current.data?.spans).toHaveLength(0);
  });

  it("reload() re-runs the fetch, for AsyncBoundary's own retry affordance", async () => {
    await activateAppLanguage("en");
    mockMatchingListRuns();
    const getTimelineSpy = vi.spyOn(executionObservabilityService, "getTimeline")
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce(timeline());
    const target = run();

    const { result } = renderTimeline(target);
    await waitFor(() => expect(result.current.error?.kind).toBe("error"));

    result.current.reload();

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.error).toBeUndefined();
    expect(getTimelineSpy).toHaveBeenCalledTimes(2);
  });
});
