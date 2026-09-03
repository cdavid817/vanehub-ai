// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { ScheduledTaskRun } from "../types/agent";
import { ScheduledTaskHistory } from "./scheduled-task-history";

function buildRun(id: string, overrides: Partial<ScheduledTaskRun> = {}): ScheduledTaskRun {
  return {
    id, taskId: "t-a", sessionId: "session-1", status: "succeeded", error: null,
    startedAt: "2026-08-31T09:00:00.000Z", completedAt: "2026-08-31T09:05:00.000Z", ...overrides,
  };
}

function loadedState(runs: ScheduledTaskRun[]): AsyncViewState<ScheduledTaskRun[]> {
  return { data: runs, initialLoading: false, refreshing: false, stale: false };
}

describe("ScheduledTaskHistory", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows a loading indicator while the initial fetch is in flight", () => {
    render(<ScheduledTaskHistory language="en" onRetry={vi.fn()} state={{ data: undefined, initialLoading: true, refreshing: false, stale: false }} />);
    expect(screen.getByRole("status")).toBeTruthy();
  });

  it("shows the empty state when there are no runs", () => {
    render(<ScheduledTaskHistory language="en" onRetry={vi.fn()} state={loadedState([])} />);
    expect(screen.getByText(i18n.t("scheduledTasks.history.empty"))).toBeTruthy();
  });

  it("shows a retryable error and calls onRetry", () => {
    const onRetry = vi.fn();
    render(<ScheduledTaskHistory language="en" onRetry={onRetry} state={{
      data: undefined, initialLoading: false, refreshing: false, stale: false,
      error: { kind: "error", message: "Scheduled task was not found.", retryable: true },
    }} />);
    expect(screen.getByText("Scheduled task was not found.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: i18n.t("featureLoad.retry") }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  // 19.11: the honest partial trigger classification -- backfilled is real and shown; a manual
  // run and a normal on-time run both render as plain "Succeeded" because the data genuinely
  // cannot tell them apart today.
  it("distinguishes a backfilled run from a normal succeeded run and a failed run", () => {
    render(<ScheduledTaskHistory language="en" onRetry={vi.fn()} state={loadedState([
      buildRun("run-succeeded", { status: "succeeded" }),
      buildRun("run-backfilled", { status: "backfilled" }),
      buildRun("run-failed", { status: "failed", error: "Agent unavailable", completedAt: "2026-08-31T09:05:00.000Z" }),
    ])} />);

    expect(within(screen.getByTestId("scheduled-task-history-row-run-succeeded")).getByText(i18n.t("scheduledTasks.history.status.succeeded"))).toBeTruthy();
    expect(within(screen.getByTestId("scheduled-task-history-row-run-backfilled")).getByText(i18n.t("scheduledTasks.history.status.backfilled"))).toBeTruthy();
    const failedRow = within(screen.getByTestId("scheduled-task-history-row-run-failed"));
    expect(failedRow.getByText(i18n.t("scheduledTasks.history.status.failed"))).toBeTruthy();
    // Safe failure: the error renders as visible text, not silently dropped.
    expect(failedRow.getByText("Agent unavailable")).toBeTruthy();
  });

  it("shows an Open session action when onOpenSession is supplied and a session exists", () => {
    const onOpenSession = vi.fn();
    render(<ScheduledTaskHistory language="en" onOpenSession={onOpenSession} onRetry={vi.fn()} state={loadedState([buildRun("run-1", { sessionId: "session-42" })])} />);
    fireEvent.click(screen.getByRole("button", { name: i18n.t("scheduledTasks.history.openSession") }));
    expect(onOpenSession).toHaveBeenCalledWith("session-42");
  });

  it("shows a dash, not a broken link, for a run with no session yet", () => {
    render(<ScheduledTaskHistory language="en" onRetry={vi.fn()} state={loadedState([buildRun("run-1", { sessionId: null, status: "skipped" })])} />);
    expect(within(screen.getByTestId("scheduled-task-history-row-run-1")).getByText("—")).toBeTruthy();
  });

  // No real pagination exists (18.6's own precedent for the identical shape) -- landing exactly on
  // the 100-row cap is the one honest signal there may be more, so it gets a bounded note; fewer
  // than 100 rows gets none, since there is nothing to honestly claim there.
  it("shows a bounded cap note only when the result lands exactly on the 100-row cap", () => {
    const { rerender } = render(<ScheduledTaskHistory language="en" onRetry={vi.fn()} state={loadedState([buildRun("run-1")])} />);
    expect(screen.queryByText(i18n.t("scheduledTasks.history.cappedNote"))).toBeNull();

    const hundredRuns = Array.from({ length: 100 }, (_unused, index) => buildRun(`run-${index}`));
    rerender(<ScheduledTaskHistory language="en" onRetry={vi.fn()} state={loadedState(hundredRuns)} />);
    expect(screen.getByText(i18n.t("scheduledTasks.history.cappedNote"))).toBeTruthy();
  });
});
