// @vitest-environment jsdom

import { useState } from "react";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { CodeReview } from "../types/code-review";
import type { ExecutionRunSummary, ExecutionTimeline } from "../types/execution-observability";
import type { MissionControlFacet, MissionControlRunDetail, MissionControlRunSummary } from "../types/mission-control";
import type { SessionLogPage } from "../types/session-workspace";
import type { TokenUsageDetailsPage, TokenUsageSummary, UsageMeasure } from "../types/token-usage";
import { MissionControlDetailPanel } from "./mission-control-detail-panel";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

const ALL_FACETS: MissionControlFacet[] = ["overview", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"];
// Session-and-review-linked, with a createdAt/endedAt window that overlaps `execRun()` below --
// unlike mission-control-facets.test.tsx's own router-focused fixture (deliberately `navigation:
// null` there, so every facet's own fetch is a no-op short-circuit that exercises routing only),
// this file needs every resolver- and navigation-backed facet's fetch to actually be reachable, since
// proving *exclusivity* between facets requires each one to have real data to fetch in the first place.
function run(): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:00:00.000Z",
    endedAt: "2026-08-16T00:10:00.000Z", projectId: null, workspace: null, phase: null, attention: null,
    reasonCode: null, verification: "unavailable", tokens: null, cost: null, actions: [],
    navigation: { kind: "review", id: "review-1", sessionId: "session-1" },
    runner: null,
  };
}

function detail(): MissionControlRunDetail {
  return { run: run(), facets: ALL_FACETS.map((facet) => ({ facet, state: "available" })) };
}

function execRun(): ExecutionRunSummary {
  return {
    runId: "exec-1", traceId: "trace-1", rootSpanId: "span-1", source: "desktop", status: "succeeded",
    startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:20:00.000Z", sessionId: "session-1",
  };
}

function timeline(): ExecutionTimeline {
  return { run: execRun(), spans: [], events: [] };
}

function logPage(): SessionLogPage {
  return { items: [], truncated: false, nextCursor: null };
}

function codeReview(): CodeReview {
  return {
    id: "review-1", sessionId: "session-1", workspaceId: "workspace-1", fingerprint: "fp-1",
    status: "active", decision: "pending", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:05:00.000Z",
    files: [], comments: [], findings: [],
    summary: { changedFiles: 0, viewedFiles: 0, unresolvedComments: 0, unresolvedFindings: 0 },
    hunkDecisions: [],
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
    totals: { reported: measure(100), reportedDerived: measure(0), estimated: measure(0) },
    userResponse: zeroTotals, internal: zeroTotals,
    counts: { calls: 1, generations: 1, sessions: 1 },
    daily: [], breakdowns: [], generatedAt: "2026-08-16T00:10:00.000Z",
  };
}

function tokenUsageDetails(): TokenUsageDetailsPage {
  return { schemaVersion: 1, invocations: [], observations: [], nextCursor: null };
}

/** Mirrors mission-control-section-nav.test.tsx's own ControlledNav pattern, but through the real
 *  detail panel -- the section nav plus the facet dispatcher plus each facet's own real fetch hook,
 *  end to end, since the property under test (16.18) is about the whole chain, not any one layer. */
function ControlledDetailPanel() {
  const [activeFacet, setActiveFacet] = useState<MissionControlFacet>("overview");
  return (
    <MissionControlDetailPanel
      activeFacet={activeFacet}
      agents={[]}
      onAct={() => undefined}
      onDismissError={() => undefined}
      onInspect={() => undefined}
      onSelectFacet={setActiveFacet}
      selected={detail()}
    />
  );
}

function selectTab(name: string) {
  fireEvent.click(screen.getByRole("tab", { name }));
}

describe("MissionControlDetailPanel facet-switching fetch exclusivity (16.18)", () => {
  it("fetches nothing at all while parked on the default Overview facet", async () => {
    await activateAppLanguage("en");
    const listRuns = vi.spyOn(executionObservabilityService, "listRuns");
    const getTimeline = vi.spyOn(executionObservabilityService, "getTimeline");
    const listSessionLogs = vi.spyOn(agentService, "listSessionLogs");
    const getCodeReview = vi.spyOn(agentService, "getCodeReview");
    const getTokenUsageSummary = vi.spyOn(agentService, "getTokenUsageSummary");

    render(<ControlledDetailPanel />);
    await waitFor(() => expect(screen.getByTestId("mission-control-overview-facet")).toBeTruthy());

    // Overview needs no join (overview-facet.tsx's own doc comment) -- confirms the shared resolver
    // itself is not called speculatively for a facet that has no use for it either.
    expect(listRuns).not.toHaveBeenCalled();
    expect(getTimeline).not.toHaveBeenCalled();
    expect(listSessionLogs).not.toHaveBeenCalled();
    expect(getCodeReview).not.toHaveBeenCalled();
    expect(getTokenUsageSummary).not.toHaveBeenCalled();
  });

  it("fetches only Review's own data when the Review tab is selected", async () => {
    await activateAppLanguage("en");
    const listRuns = vi.spyOn(executionObservabilityService, "listRuns");
    const getTimeline = vi.spyOn(executionObservabilityService, "getTimeline");
    const listSessionLogs = vi.spyOn(agentService, "listSessionLogs");
    const getCodeReview = vi.spyOn(agentService, "getCodeReview").mockResolvedValue(codeReview());
    const getTokenUsageSummary = vi.spyOn(agentService, "getTokenUsageSummary");

    render(<ControlledDetailPanel />);
    selectTab("Review");

    await waitFor(() => expect(getCodeReview).toHaveBeenCalledTimes(1));
    expect(getCodeReview).toHaveBeenCalledWith("review-1");
    // Review needs no session-execution join at all (use-mission-control-review.ts's own doc
    // comment) -- the shared resolver never fires for it, unlike Logs/Usage/Timeline below.
    expect(listRuns).not.toHaveBeenCalled();
    expect(getTimeline).not.toHaveBeenCalled();
    expect(listSessionLogs).not.toHaveBeenCalled();
    expect(getTokenUsageSummary).not.toHaveBeenCalled();
  });

  it("fetches only Logs's own data when the Logs tab is selected", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({ items: [execRun()], nextPageToken: null });
    const getTimeline = vi.spyOn(executionObservabilityService, "getTimeline");
    const listSessionLogs = vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage());
    const getCodeReview = vi.spyOn(agentService, "getCodeReview");
    const getTokenUsageSummary = vi.spyOn(agentService, "getTokenUsageSummary");

    render(<ControlledDetailPanel />);
    selectTab("Logs");

    await waitFor(() => expect(listSessionLogs).toHaveBeenCalledTimes(1));
    expect(listSessionLogs).toHaveBeenCalledWith(expect.objectContaining({ sessionId: "session-1", runId: "exec-1" }));
    expect(getTimeline).not.toHaveBeenCalled();
    expect(getCodeReview).not.toHaveBeenCalled();
    expect(getTokenUsageSummary).not.toHaveBeenCalled();
  });

  it("fetches only Usage's own data when the Usage tab is selected", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({ items: [execRun()], nextPageToken: null });
    const getTimeline = vi.spyOn(executionObservabilityService, "getTimeline");
    const listSessionLogs = vi.spyOn(agentService, "listSessionLogs");
    const getCodeReview = vi.spyOn(agentService, "getCodeReview");
    const getTokenUsageSummary = vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(tokenUsageSummary());
    const getTokenUsageDetails = vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(tokenUsageDetails());

    render(<ControlledDetailPanel />);
    selectTab("Usage");

    await waitFor(() => expect(getTokenUsageSummary).toHaveBeenCalledTimes(1));
    expect(getTokenUsageDetails).toHaveBeenCalledTimes(1);
    expect(getTimeline).not.toHaveBeenCalled();
    expect(listSessionLogs).not.toHaveBeenCalled();
    expect(getCodeReview).not.toHaveBeenCalled();
  });

  it("fetches only the shared Timeline data when the Timeline tab is selected, not Logs/Usage/Review", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({ items: [execRun()], nextPageToken: null });
    const getTimeline = vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline());
    const listSessionLogs = vi.spyOn(agentService, "listSessionLogs");
    const getCodeReview = vi.spyOn(agentService, "getCodeReview");
    const getTokenUsageSummary = vi.spyOn(agentService, "getTokenUsageSummary");

    render(<ControlledDetailPanel />);
    selectTab("Timeline");

    await waitFor(() => expect(getTimeline).toHaveBeenCalledTimes(1));
    expect(getTimeline).toHaveBeenCalledWith("exec-1");
    expect(listSessionLogs).not.toHaveBeenCalled();
    expect(getCodeReview).not.toHaveBeenCalled();
    expect(getTokenUsageSummary).not.toHaveBeenCalled();
  });

  it("does not re-fetch a facet already left, nor jump ahead to one not yet visited, across a Logs -> Usage -> Review walk", async () => {
    await activateAppLanguage("en");
    vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({ items: [execRun()], nextPageToken: null });
    const getTimeline = vi.spyOn(executionObservabilityService, "getTimeline");
    const listSessionLogs = vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage());
    const getCodeReview = vi.spyOn(agentService, "getCodeReview").mockResolvedValue(codeReview());
    const getTokenUsageSummary = vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(tokenUsageSummary());
    vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(tokenUsageDetails());

    render(<ControlledDetailPanel />);

    selectTab("Logs");
    await waitFor(() => expect(listSessionLogs).toHaveBeenCalledTimes(1));

    selectTab("Usage");
    await waitFor(() => expect(getTokenUsageSummary).toHaveBeenCalledTimes(1));
    expect(listSessionLogs).toHaveBeenCalledTimes(1); // unmounted, not re-fetched on the way out

    selectTab("Review");
    await waitFor(() => expect(getCodeReview).toHaveBeenCalledTimes(1));
    expect(listSessionLogs).toHaveBeenCalledTimes(1);
    expect(getTokenUsageSummary).toHaveBeenCalledTimes(1);
    expect(getTimeline).not.toHaveBeenCalled(); // never visited on this walk
  });
});
