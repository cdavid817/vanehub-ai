// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary, ExecutionSpanSummary, ExecutionTimeline } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import { ToolsFacet } from "./tools-facet";

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

describe("ToolsFacet", () => {
  it("keeps only tool-kind spans, excluding mcp, file, and other kinds from the same timeline", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline([
      span({ spanId: "s-tool", name: "bash: ls", kind: "tool" }),
      span({ spanId: "s-mcp", name: "mcp: search", kind: "mcp" }),
      span({ spanId: "s-file", name: "write: notes.md", kind: "file" }),
      span({ spanId: "s-model", name: "chat completion", kind: "model" }),
    ]));

    render(<ToolsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("bash: ls")).toBeTruthy());
    // The filter is proven real by what is absent, not just by what is present.
    expect(screen.queryByText("mcp: search")).toBeNull();
    expect(screen.queryByText("write: notes.md")).toBeNull();
    expect(screen.queryByText("chat completion")).toBeNull();
  });

  it("does not repeat a kind badge on its rows, since every row is already known to be a tool call", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline([span({ name: "bash: ls", kind: "tool" })]));

    render(<ToolsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("bash: ls")).toBeTruthy());
    expect(screen.queryByText("Tool")).toBeNull();
  });

  it("renders the noSpans state when the run resolved and has spans, but none of kind tool", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(executionObservabilityService, "getTimeline").mockResolvedValue(timeline([span({ name: "chat completion", kind: "model" })]));

    render(<ToolsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("No tool calls were recorded for this Run.")).toBeTruthy());
    expect(screen.queryByText("chat completion")).toBeNull();
    // Distinct from the resolver-empty state — this Run has data, just none of this facet's kind.
    expect(screen.queryByText("No linked execution data for this Run.")).toBeNull();
  });

  it("renders the empty state, without fetching a timeline, when the resolver finds nothing", async () => {
    await activateAppLanguage("en");
    const getTimelineSpy = vi.spyOn(executionObservabilityService, "getTimeline");

    render(<ToolsFacet run={run({ navigation: null })} />);

    await waitFor(() => expect(screen.getByText("No linked execution data for this Run.")).toBeTruthy());
    expect(getTimelineSpy).not.toHaveBeenCalled();
  });

  it("shows a safe error and does not leak backend diagnostics when tool calls cannot be loaded", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(executionObservabilityService, "getTimeline").mockRejectedValue(new Error("token=secret"));

    render(<ToolsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("Could not load tool calls for this Run.")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
  });

  it("loads and translates every locale's new tools-facet strings, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      for (const key of ["loading", "empty", "error", "noSpans"]) {
        expect(t(`missionControl.tools.${key}`)).not.toBe(`missionControl.tools.${key}`);
      }
    }
  });
});
