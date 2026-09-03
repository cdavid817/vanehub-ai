// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { AgentRegistryEntry, ScheduledTask, ScheduledTaskRun } from "../types/agent";
import { ScheduledTaskDetail } from "./scheduled-task-detail";

const weekdayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const emptyHistory: AsyncViewState<ScheduledTaskRun[]> = { data: [], initialLoading: false, refreshing: false, stale: false };

function buildTask(overrides: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    id: "t-a", name: "Nightly digest", content: "Summarize commits", agentId: "codex-cli",
    frequency: { kind: "daily", timeOfDay: "09:00" }, enabled: true, nextRunAt: "2026-08-31T09:00:00.000Z",
    latestStatus: "never-run", latestRunAt: null, latestRunSessionId: null, latestError: null,
    createdAt: "2026-08-01T00:00:00.000Z", updatedAt: "2026-08-01T00:00:00.000Z", version: 1, ...overrides,
  };
}

function buildAgent(overrides: Partial<AgentRegistryEntry> = {}): AgentRegistryEntry {
  return { id: "codex-cli", displayName: "Codex CLI", supportedInteractionModes: ["cli"], availabilityState: "available", ...overrides } as AgentRegistryEntry;
}

function renderDetail(props: Partial<Parameters<typeof ScheduledTaskDetail>[0]> = {}) {
  return render(
    <ScheduledTaskDetail
      agent={buildAgent()}
      history={emptyHistory}
      isRunningNow={false}
      language="en"
      onRetryHistory={vi.fn()}
      onRunNow={vi.fn()}
      runNowError={null}
      task={buildTask()}
      weekdayNames={weekdayNames}
      {...props}
    />,
  );
}

describe("ScheduledTaskDetail", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows the empty placeholder when no task is selected", () => {
    render(<ScheduledTaskDetail agent={undefined} history={emptyHistory} isRunningNow={false} language="en" onRetryHistory={vi.fn()} onRunNow={vi.fn()} runNowError={null} task={null} weekdayNames={weekdayNames} />);
    expect(screen.getByText(i18n.t("scheduledTasks.detailEmpty"))).toBeTruthy();
  });

  // 19.6: composes 19.3's already-correct configuration fields with the four pieces that
  // placeholder deferred -- this pins that all four actually appear together in the real detail
  // view, not just in each sub-component's own isolated test.
  it("renders configuration, occurrence preview, capability notice, latest run, and history together", () => {
    renderDetail();
    const detail = screen.getByTestId("scheduled-task-detail");
    expect(within(detail).getByText("Nightly digest")).toBeTruthy();
    expect(within(detail).getByTestId("scheduled-task-occurrences")).toBeTruthy();
    expect(within(detail).getByTestId("scheduled-task-capability-notice")).toBeTruthy();
    expect(within(detail).getByTestId("scheduled-task-latest-run")).toBeTruthy();
    expect(within(detail).getByTestId("scheduled-task-history")).toBeTruthy();
  });

  it("shows an honest 'not run yet' message when the task has never run", () => {
    renderDetail({ task: buildTask({ latestRunAt: null }) });
    const latestRun = screen.getByTestId("scheduled-task-latest-run");
    expect(within(latestRun).getByText(i18n.t("scheduledTasks.latestRun.none"))).toBeTruthy();
  });

  // "Latest Run" extends the fields the task already carries (19.6's own "extend/integrate rather
  // than duplicate") -- no separate service call, since these already arrived with the task.
  it("shows the latest run's own timestamp, session, and error once the task has run", () => {
    renderDetail({ task: buildTask({ latestRunAt: "2026-08-30T09:00:00.000Z", latestRunSessionId: "session-9", latestStatus: "failed", latestError: "Agent unavailable" }) });
    const latestRun = screen.getByTestId("scheduled-task-latest-run");
    expect(within(latestRun).getByText("Agent unavailable")).toBeTruthy();
    expect(within(latestRun).getByText("session-9")).toBeTruthy();
  });

  it("flags a missing Agent inside the capability notice when the agent id no longer resolves", () => {
    renderDetail({ agent: undefined, task: buildTask({ agentId: "removed-agent" }) });
    expect(screen.getByText(i18n.t("scheduledTasks.capability.agentMissing", { agentId: "removed-agent" }))).toBeTruthy();
  });

  it("passes the history state through to ScheduledTaskHistory, including a failed row's error", () => {
    renderDetail({
      history: {
        data: [{ id: "run-1", taskId: "t-a", sessionId: null, status: "failed", error: "Timed out", startedAt: "2026-08-30T09:00:00.000Z", completedAt: "2026-08-30T09:05:00.000Z" }],
        initialLoading: false, refreshing: false, stale: false,
      },
    });
    expect(within(screen.getByTestId("scheduled-task-history")).getByText("Timed out")).toBeTruthy();
  });
});
