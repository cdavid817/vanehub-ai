// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary, ExecutionSpanSummary, ExecutionTimeline } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import { TimelineFacet } from "./timeline-facet";

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
    startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:20:00.000Z", sessionId: "session-1",
    ...overrides,
  };
}

function span(overrides: Partial<ExecutionSpanSummary> = {}): ExecutionSpanSummary {
  return {
    spanId: "span-1", parentSpanId: null, name: "span-1", kind: "tool", status: "succeeded",
    fidelity: "native", startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:00:01.000Z",
    durationMs: 1000, errorClassification: null, attributes: {}, depth: 0,
    startOffsetMs: 0, completedDurationMs: 1000, delegated: false, criticalPath: false, links: [],
    ...overrides,
  };
}

function timeline(spans: ExecutionSpanSummary[]): ExecutionTimeline {
  return { run: execRun(), spans, events: [] };
}

function mockMatchingRun() {
  return vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({
    items: [execRun()],
    nextPageToken: null,
  });
}

describe("TimelineFacet", () => {
  it("renders every span regardless of kind, with each one's own kind badge and critical-path marker", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline([
      span({ spanId: "s-tool", name: "bash: ls", kind: "tool" }),
      span({ spanId: "s-mcp", name: "mcp: search", kind: "mcp" }),
      span({ spanId: "s-file", name: "write: notes.md", kind: "file", criticalPath: true }),
    ]));

    render(<TimelineFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("bash: ls")).toBeTruthy());
    expect(screen.getByText("mcp: search")).toBeTruthy();
    expect(screen.getByText("write: notes.md")).toBeTruthy();
    expect(screen.getByText("Tool")).toBeTruthy();
    expect(screen.getByText("MCP")).toBeTruthy();
    expect(screen.getByText("File")).toBeTruthy();
    expect(screen.getByTitle("On the critical path")).toBeTruthy();
  });

  it("renders the noSpans state when the run resolved but its timeline has zero spans", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline([]));

    render(<TimelineFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("This Run recorded no spans.")).toBeTruthy());
    // Distinct copy from the resolver-empty state below — a real ready state, not a failed join.
    expect(screen.queryByText("No linked execution data for this Run.")).toBeNull();
  });

  it("renders the empty state, without fetching a timeline, when the resolver finds nothing", async () => {
    await activateAppLanguage("en");
    const getTimelineSpy = vi.spyOn(executionObservabilityService, "getTimeline");

    render(<TimelineFacet run={run({ navigation: null })} />);

    await waitFor(() => expect(screen.getByText("No linked execution data for this Run.")).toBeTruthy());
    expect(getTimelineSpy).not.toHaveBeenCalled();
  });

  it("shows a safe error and does not leak backend diagnostics when the timeline cannot be loaded", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(executionObservabilityService, "getTimeline").mockRejectedValue(new Error("token=secret"));

    render(<TimelineFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("Could not load the timeline for this Run.")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
  });

  it("loads and translates every locale's new timeline-facet strings, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      for (const key of ["loading", "empty", "error", "noSpans"]) {
        expect(t(`missionControl.timeline.${key}`)).not.toBe(`missionControl.timeline.${key}`);
      }
    }
  });
});
