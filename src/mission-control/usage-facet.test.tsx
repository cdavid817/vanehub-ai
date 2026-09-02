// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { formatAppNumber } from "../i18n/format";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { ModelInvocation, TokenUsageDetailsPage, TokenUsageSummary, UsageMeasure } from "../types/token-usage";
import { UsageFacet } from "./usage-facet";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

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

describe("UsageFacet", () => {
  it("renders real usage totals and recent model calls once the resolver finds a matching execution run", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:20:00.000Z" })],
      nextPageToken: null,
    });
    const summarySpy = vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(tokenUsageSummary());
    const detailsSpy = vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPage([invocation()]));

    render(<UsageFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("claude-sonnet")).toBeTruthy());
    expect(summarySpy).toHaveBeenCalledWith({ sessionId: "session-1" });
    expect(detailsSpy).toHaveBeenCalledWith({ sessionId: "session-1", limit: 20 });
    expect(screen.getByText(formatAppNumber(4_200, "en", { maximumFractionDigits: 1 }))).toBeTruthy();
    expect(screen.getByText("Succeeded · Initial response")).toBeTruthy();
  });

  it("renders the empty state without querying usage when the run has no session link", async () => {
    await activateAppLanguage("en");
    const summarySpy = vi.spyOn(agentService, "getTokenUsageSummary");

    render(<UsageFacet run={run({ navigation: null })} />);

    await waitFor(() => expect(screen.getByText("No linked execution data for this Run.")).toBeTruthy());
    expect(summarySpy).not.toHaveBeenCalled();
  });

  it("renders the empty state when zero execution runs overlap the Mission Control run's window", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2020-01-01T00:00:00.000Z", endedAt: "2020-01-01T00:05:00.000Z" })],
      nextPageToken: null,
    });

    render(<UsageFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("No linked execution data for this Run.")).toBeTruthy());
  });

  it("shows a safe error and does not leak backend diagnostics when usage cannot be loaded", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:20:00.000Z" })],
      nextPageToken: null,
    });
    vi.spyOn(agentService, "getTokenUsageSummary").mockRejectedValue(new Error("token=secret"));
    vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPage([]));

    render(<UsageFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("Could not load usage for this Run.")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
  });

  it("shows the no-invocations fallback when the summary loads but no model calls were recorded", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
      items: [execRun({ startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:20:00.000Z" })],
      nextPageToken: null,
    });
    vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(tokenUsageSummary());
    vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPage([]));

    render(<UsageFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("No model calls recorded.")).toBeTruthy());
  });

  it("loads and translates every locale's new usage-facet strings, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      for (const key of ["empty", "error", "invocations", "noInvocations", "unknownModel"]) {
        expect(t(`missionControl.usage.${key}`)).not.toBe(`missionControl.usage.${key}`);
      }
    }
  });
});
