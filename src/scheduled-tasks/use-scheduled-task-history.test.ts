// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ScheduledTaskRun, ScheduledTaskRunPage } from "../types/agent";

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

function buildPage(items: ScheduledTaskRun[], nextCursor: string | null = null): ScheduledTaskRunPage {
  return { items, nextCursor };
}

describe("useScheduledTaskHistory", () => {
  beforeEach(() => { mocks.listScheduledTaskRuns.mockReset(); });

  it("does not fetch and reports no loading when there is no selected task", () => {
    const { result } = renderHook(() => useScheduledTaskHistory(null));
    expect(mocks.listScheduledTaskRuns).not.toHaveBeenCalled();
    expect(result.current).toMatchObject({ data: undefined, initialLoading: false, hasMore: false, loadingMore: false });
  });

  it("fetches history for the selected task id and reports it once resolved", async () => {
    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-1"), buildRun("run-2")]));
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
    let resolveTaskA: (value: ScheduledTaskRunPage) => void = () => {};
    mocks.listScheduledTaskRuns.mockImplementationOnce(() => new Promise((resolve) => { resolveTaskA = resolve; }));
    const { rerender, result } = renderHook(({ taskId }) => useScheduledTaskHistory(taskId), { initialProps: { taskId: "t-a" } });

    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-b", { taskId: "t-b" })]));
    rerender({ taskId: "t-b" });
    await waitFor(() => expect(result.current.data?.[0]?.id).toBe("run-b"));

    // Task A's slow response resolves after B's already landed -- it must not overwrite B's data.
    resolveTaskA(buildPage([buildRun("run-a", { taskId: "t-a" })]));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(result.current.data?.[0]?.id).toBe("run-b");
  });

  it("reload() re-fetches for the current task id", async () => {
    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-1")]));
    const { result } = renderHook(() => useScheduledTaskHistory("t-a"));
    await waitFor(() => expect(result.current.data).toBeTruthy());

    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-1"), buildRun("run-2")]));
    result.current.reload();
    await waitFor(() => expect(result.current.data).toHaveLength(2));
    expect(mocks.listScheduledTaskRuns).toHaveBeenCalledTimes(2);
  });

  // 19.11: the real gap this task closed -- `hasMore`/`loadMore` did not exist at all before this
  // pass, so a reader could never reach anything past whatever the first page returned.
  it("reports hasMore from the resolved page's nextCursor, and loadMore appends the next page", async () => {
    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-1")], "1"));
    const { result } = renderHook(() => useScheduledTaskHistory("t-a"));
    await waitFor(() => expect(result.current.data).toBeTruthy());
    expect(result.current.hasMore).toBe(true);

    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-2")], null));
    await act(async () => { result.current.loadMore(); });

    expect(mocks.listScheduledTaskRuns).toHaveBeenLastCalledWith("t-a", { cursor: "1" });
    expect(result.current.data?.map((run) => run.id)).toEqual(["run-1", "run-2"]);
    expect(result.current.hasMore).toBe(false);
    expect(result.current.loadingMore).toBe(false);
  });

  it("loadMore is a no-op once there is no further page", async () => {
    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-1")], null));
    const { result } = renderHook(() => useScheduledTaskHistory("t-a"));
    await waitFor(() => expect(result.current.data).toBeTruthy());

    mocks.listScheduledTaskRuns.mockClear();
    await act(async () => { result.current.loadMore(); });
    expect(mocks.listScheduledTaskRuns).not.toHaveBeenCalled();
  });

  it("loadMore is a no-op while an earlier loadMore is still in flight", async () => {
    mocks.listScheduledTaskRuns.mockResolvedValueOnce(buildPage([buildRun("run-1")], "1"));
    const { result } = renderHook(() => useScheduledTaskHistory("t-a"));
    await waitFor(() => expect(result.current.data).toBeTruthy());

    let resolveSecondPage: (value: ScheduledTaskRunPage) => void = () => {};
    mocks.listScheduledTaskRuns.mockImplementationOnce(() => new Promise((resolve) => { resolveSecondPage = resolve; }));
    act(() => { result.current.loadMore(); });
    await waitFor(() => expect(result.current.loadingMore).toBe(true));

    mocks.listScheduledTaskRuns.mockClear();
    act(() => { result.current.loadMore(); });
    expect(mocks.listScheduledTaskRuns).not.toHaveBeenCalled();

    await act(async () => { resolveSecondPage(buildPage([buildRun("run-2")], null)); });
  });
});
