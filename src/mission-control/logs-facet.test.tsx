// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionRunSummary } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { SessionLogEntry, SessionLogPage } from "../types/session-workspace";
import { LogsFacet } from "./logs-facet";

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

function mockMatchingRun() {
  return vi.spyOn(executionObservabilityService, "listRuns").mockResolvedValue({ items: [execRun()], nextPageToken: null });
}

describe("LogsFacet", () => {
  it("renders bounded log entries once the resolver finds a matching execution run", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage([
      logEntry({ id: "log-1", level: "error", message: "process exited" }),
      logEntry({ id: "log-2", level: "info", message: "resumed" }),
    ]));

    render(<LogsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("process exited")).toBeTruthy());
    expect(screen.getByText("resumed")).toBeTruthy();
    expect(screen.getByTestId("mission-control-logs-entries")).toBeTruthy();
  });

  it("discloses a bounded, non-exhaustive view when the page is truncated", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage([logEntry()], { truncated: true }));

    render(<LogsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText(/Only the most recent 20 entries are shown\./)).toBeTruthy());
  });

  it("does not show a capped note when the page is not truncated", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage([logEntry()], { truncated: false }));

    render(<LogsFacet run={run()} />);

    await waitFor(() => expect(screen.getByTestId("mission-control-logs-entries")).toBeTruthy());
    expect(screen.queryByText(/Only the most recent/)).toBeNull();
  });

  it("renders the no-entries state when the run resolved but zero log entries were recorded", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockResolvedValue(logPage([]));

    render(<LogsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("No log entries were recorded for this Run.")).toBeTruthy());
    // Distinct copy from the resolver-unavailable state below — a real ready state, not a failed join.
    expect(screen.queryByText("No linked execution data for this Run.")).toBeNull();
  });

  it("renders the unavailable state, without fetching logs, when the resolver finds nothing", async () => {
    await activateAppLanguage("en");
    const listSpy = vi.spyOn(agentService, "listSessionLogs");

    render(<LogsFacet run={run({ navigation: null })} />);

    await waitFor(() => expect(screen.getByText("No linked execution data for this Run.")).toBeTruthy());
    expect(listSpy).not.toHaveBeenCalled();
  });

  it("shows a safe error and does not leak backend diagnostics when logs cannot be loaded", async () => {
    await activateAppLanguage("en");
    mockMatchingRun();
    vi.spyOn(agentService, "listSessionLogs").mockRejectedValue(new Error("token=secret"));

    render(<LogsFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("Could not load logs for this Run.")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
  });

  it("loads and translates every locale's new logs-facet strings, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      for (const key of ["empty", "error", "noEntries", "cappedNote"]) {
        expect(t(`missionControl.logs.${key}`)).not.toBe(`missionControl.logs.${key}`);
      }
    }
  });
});
