// @vitest-environment jsdom

import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ScheduledTaskRun } from "../types/agent";

const mocks = vi.hoisted(() => ({ listScheduledTaskRuns: vi.fn() }));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: { listScheduledTaskRuns: mocks.listScheduledTaskRuns },
}));

import { useScheduledTaskHistory } from "./use-scheduled-task-history";

function buildRun(id: string, overrides: Partial<ScheduledTaskRun> = {}): ScheduledTaskRun {
  return {
    id, taskId: "t-a", sessionId: "session-1", status: "succeeded", error: null,
    startedAt: "2026-08-31T09:00:00.000Z", completedAt: "2026-08-31T09:00:00.000Z", ...overrides,
  };
}

describe("useScheduledTaskHistory", () => {
  beforeEach(() => { mocks.listScheduledTaskRuns.mockReset(); });

  it("does not fetch and reports no loading when there is no selected task", () => {
    const { result } = renderHook(() => useScheduledTaskHistory(null));
    expect(mocks.listScheduledTaskRuns).not.toHaveBeenCalled();
    expect(result.current).toMatchObject({ data: undefined, initialLoading: false });
  });

  it("fetches history for the selected task id and reports it once resolved", async () => {
    mocks.listScheduledTaskRuns.mockResolvedValueOnce([buildRun("run-1"), buildRun("run-2")]);
    const { result } = renderHook(() => useScheduledTaskHistory("t-a"));

    expect(result.current.initialLoading).toBe(true);
    await waitFor(() => expect(result.current.initialLoading).toBe(false));
    expect(mocks.listScheduledTaskRuns).toHaveBeenCalledWith("t-a");
    expect(result.current.data?.map((run) => run.id)).toEqual(["run-1", "run-2"]);
  });

  it("surfaces a rejected fetch as a retryable DisplayableError", async () => {
    mocks.listScheduledTaskRuns.mockRejectedValueOnce(new Error("Scheduled task was not found."));
    const { result } = renderHook(() => useScheduledTaskHistory("t-a"));

    await waitFor(() => expect(result.current.error).toBeTruthy());
    expect(result.current.error).toMatchObject({ kind: "error", message: "Scheduled task was not found.", retryable: true });
  });

  // The core race this hook exists to prevent: a fast reselect must not let a slow response for a
  // task the reader has since navigated away from land after a newer one already resolved --
  // mirrors `use-session-logs.ts`'s own `generation`-guarded `loadFirstPage`.
  it("ignores a stale response from a task the reader has since navigated away from", async () => {
    let resolveTaskA: (value: ScheduledTaskRun[]) => void = () => {};
    mocks.listScheduledTaskRuns.mockImplementationOnce(() => new Promise((resolve) => { resolveTaskA = resolve; }));
    const { rerender, result } = renderHook(({ taskId }) => useScheduledTaskHistory(taskId), { initialProps: { taskId: "t-a" } });

    mocks.listScheduledTaskRuns.mockResolvedValueOnce([buildRun("run-b", { taskId: "t-b" })]);
    rerender({ taskId: "t-b" });
    await waitFor(() => expect(result.current.data?.[0]?.id).toBe("run-b"));

    // Task A's slow response resolves after B's already landed -- it must not overwrite B's data.
    resolveTaskA([buildRun("run-a", { taskId: "t-a" })]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(result.current.data?.[0]?.id).toBe("run-b");
  });

  it("reload() re-fetches for the current task id", async () => {
    mocks.listScheduledTaskRuns.mockResolvedValueOnce([buildRun("run-1")]);
    const { result } = renderHook(() => useScheduledTaskHistory("t-a"));
    await waitFor(() => expect(result.current.data).toBeTruthy());

    mocks.listScheduledTaskRuns.mockResolvedValueOnce([buildRun("run-1"), buildRun("run-2")]);
    result.current.reload();
    await waitFor(() => expect(result.current.data).toHaveLength(2));
    expect(mocks.listScheduledTaskRuns).toHaveBeenCalledTimes(2);
  });
});
