// @vitest-environment jsdom

import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { SessionLogEntry, SessionLogPage, SessionLogQuery } from "../types/session-workspace";
import { useMissionControlLogs } from "./use-mission-control-logs";

afterEach(() => vi.restoreAllMocks());

const UNAVAILABLE_MESSAGE = "test-unavailable-message";
const ERROR_MESSAGE = "test-error-message";

function renderLogs(target: MissionControlRunSummary) {
  return renderHook(() => useMissionControlLogs(target, UNAVAILABLE_MESSAGE, ERROR_MESSAGE));
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
    startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:20:00.000Z", sessionId: "session-1",
    ...overrides,
  };
}

function mockMatchingRun() {
  return vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({ items: [execRun()], nextPageToken: null });
}

function logEntry(overrides: Partial<SessionLogEntry> = {}): SessionLogEntry {
  return {
    id: "log-1", timestamp: "2026-08-16T00:01:00.000Z", level: "info", category: "agent_runtime",
    message: "started", context: {},
    ...overrides,
  };
}

function logPage(items: SessionLogEntry[], overrides: Partial<SessionLogPage> = {}): SessionLogPage {
  return { items, truncated: false, nextCursor: null, ...overrides };
}

describe("useMissionControlLogs", () => {
  it("starts loading with no data before the resolver settles", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage([]));

    const { result } = renderLogs(run());

    expect(result.current.initialLoading).toBe(true);
    expect(result.current.data).toBeUndefined();
  });

  it("queries a bounded, run-correlated page once the resolver finds a matching execution run", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    const listSpy = vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage([logEntry()]));

    const { result } = renderLogs(run());

    await waitFor(() => expect(result.current.data).toBeDefined());
    const query = listSpy.mock.calls[0][0] as SessionLogQuery;
    expect(query.sessionId).toBe("session-1");
    expect(query.runId).toBe("exec-1");
    // Bounded: a real limit is always sent, never an unbounded/absent one (16.12).
    expect(typeof query.limit).toBe("number");
    expect(query.limit).toBeGreaterThan(0);
    expect(result.current.data?.entries).toHaveLength(1);
    expect(result.current.error).toBeUndefined();
  });

  it("resolves to an unavailable state, without querying logs, when the run has no session link", async () => {
    await activateAppLanguage("en");
    const listSpy = vi.spyOn(agentService, "listSessionLogs");

    const { result } = renderLogs(run({ navigation: null }));

    await waitFor(() => expect(result.current.error?.kind).toBe("unavailable"));
    expect(result.current.error?.message).toBe(UNAVAILABLE_MESSAGE);
    expect(listSpy).not.toHaveBeenCalled();
  });

  it("resolves to an unavailable state when zero execution runs overlap the Mission Control run's window", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2020-01-01T00:00:00.000Z", endedAt: "2020-01-01T00:05:00.000Z" })],
      nextPageToken: null,
    });

    const { result } = renderLogs(run());

    await waitFor(() => expect(result.current.error?.kind).toBe("unavailable"));
  });

  it("carries the page's own truncated flag through, so the facet can disclose a bounded view honestly", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage([logEntry()], { truncated: true }));

    const { result } = renderLogs(run());

    await waitFor(() => expect(result.current.data?.truncated).toBe(true));
  });

  it("resolves to a retryable error, without leaking the raw reason, when logs cannot be loaded", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockRejectedValue(new Error("token=secret"));

    const { result } = renderLogs(run());

    await waitFor(() => expect(result.current.error?.kind).toBe("error"));
    expect(result.current.error?.retryable).toBe(true);
    expect(result.current.error?.message).toBe(ERROR_MESSAGE);
  });

  it("reload() re-runs the fetch, for AsyncBoundary's own retry affordance", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    const listSpy = vi.spyOn(agentService, "listSessionLogs")
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce(logPage([logEntry()]));

    const { result } = renderLogs(run());
    await waitFor(() => expect(result.current.error?.kind).toBe("error"));

    result.current.reload();

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.error).toBeUndefined();
    expect(listSpy).toHaveBeenCalledTimes(2);
  });
});
