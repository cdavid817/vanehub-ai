// @vitest-environment jsdom

import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { ModelInvocation, TokenUsageDetailsPage, TokenUsageSummary, UsageMeasure } from "../types/token-usage";
import { useMissionControlUsage } from "./use-mission-control-usage";

afterEach(() => vi.restoreAllMocks());

const UNAVAILABLE_MESSAGE = "test-unavailable-message";
const ERROR_MESSAGE = "test-error-message";

function renderUsage(target: MissionControlRunSummary) {
  return renderHook(() => useMissionControlUsage(target, UNAVAILABLE_MESSAGE, ERROR_MESSAGE));
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

function measure(headlineTotal: number | null): UsageMeasure {
  return {
    unit: "tokens",
    dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: null },
    headlineTotal, callCount: 0, observationCount: 0,
  };
}

function tokenUsageSummary(): TokenUsageSummary {
  const zeroTotals = { reported: measure(0), reportedDerived: measure(0), estimated: measure(0) };
  return {
    schemaVersion: 1,
    totals: { reported: measure(4_200), reportedDerived: measure(0), estimated: measure(0) },
    userResponse: zeroTotals, internal: zeroTotals,
    counts: { calls: 3, generations: 2, sessions: 1 },
    daily: [], breakdowns: [], generatedAt: "2026-08-16T00:10:00.000Z",
  };
}

function invocation(overrides: Partial<ModelInvocation> = {}): ModelInvocation {
  return {
    id: "invocation-1", generationId: null, runId: null, operationId: null, sessionId: "session-1",
    messageId: null, agentId: "claude-code", providerId: "anthropic", profileId: null, endpointId: null,
    modelId: "claude-sonnet", interactionKind: "managed-cli", purpose: "assistant-initial",
    requestSequence: 1, attempt: 1, status: "succeeded",
    startedAt: "2026-08-16T00:01:00.000Z", completedAt: "2026-08-16T00:01:05.000Z",
    ...overrides,
  };
}

function detailsPage(invocations: ModelInvocation[]): TokenUsageDetailsPage {
  return { schemaVersion: 1, invocations, observations: [], nextCursor: null };
}

describe("useMissionControlUsage", () => {
  it("starts loading with no data before the resolver settles", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(tokenUsageSummary());
    vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPage([]));

    const { result } = renderUsage(run());

    expect(result.current.initialLoading).toBe(true);
    expect(result.current.data).toBeUndefined();
  });

  it("fetches usage by the resolved execution run's own sessionId once the resolver finds a match", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    const summarySpy = vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(tokenUsageSummary());
    const detailsSpy = vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPage([invocation()]));

    const { result } = renderUsage(run());

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(summarySpy).toHaveBeenCalledWith({ sessionId: "session-1" });
    expect(detailsSpy).toHaveBeenCalledWith({ sessionId: "session-1", limit: 20 });
    expect(result.current.data?.summary.totals.reported.headlineTotal).toBe(4_200);
    expect(result.current.data?.invocations).toHaveLength(1);
    expect(result.current.error).toBeUndefined();
  });

  it("resolves to an unavailable state, without querying usage, when the run has no session link", async () => {
    await activateAppLanguage("en");
    const summarySpy = vi.spyOn(agentService, "getTokenUsageSummary");

    const { result } = renderUsage(run({ navigation: null }));

    await waitFor(() => expect(result.current.error?.kind).toBe("unavailable"));
    expect(result.current.error?.message).toBe(UNAVAILABLE_MESSAGE);
    expect(summarySpy).not.toHaveBeenCalled();
  });

  it("resolves to an unavailable state when zero execution runs overlap the Mission Control run's window", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2020-01-01T00:00:00.000Z", endedAt: "2020-01-01T00:05:00.000Z" })],
      nextPageToken: null,
    });

    const { result } = renderUsage(run());

    await waitFor(() => expect(result.current.error?.kind).toBe("unavailable"));
  });

  it("resolves to a retryable error, without leaking the raw reason, when usage cannot be loaded", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "getTokenUsageSummary").mockRejectedValue(new Error("token=secret"));
    vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPage([]));

    const { result } = renderUsage(run());

    await waitFor(() => expect(result.current.error?.kind).toBe("error"));
    expect(result.current.error?.retryable).toBe(true);
    expect(result.current.error?.message).toBe(ERROR_MESSAGE);
  });

  it("reload() re-runs the fetch, for AsyncBoundary's own retry affordance", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    const summarySpy = vi.spyOn(agentService, "getTokenUsageSummary")
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce(tokenUsageSummary());
    vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPage([]));

    const { result } = renderUsage(run());
    await waitFor(() => expect(result.current.error?.kind).toBe("error"));

    result.current.reload();

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.error).toBeUndefined();
    expect(summarySpy).toHaveBeenCalledTimes(2);
  });
});
