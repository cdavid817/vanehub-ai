// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { ScheduledTask } from "../types/agent";

const mocks = vi.hoisted(() => ({
  createScheduledTask: vi.fn(),
  deleteScheduledTask: vi.fn(),
  listScheduledTasks: vi.fn(),
  runScheduledTaskNow: vi.fn(),
  setScheduledTaskEnabled: vi.fn(),
  updateScheduledTask: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    createScheduledTask: mocks.createScheduledTask,
    deleteScheduledTask: mocks.deleteScheduledTask,
    listScheduledTasks: mocks.listScheduledTasks,
    runScheduledTaskNow: mocks.runScheduledTaskNow,
    setScheduledTaskEnabled: mocks.setScheduledTaskEnabled,
    updateScheduledTask: mocks.updateScheduledTask,
  },
}));

import { isScheduledTaskVersionConflict, SCHEDULED_TASK_CREATE_MUTATION_KEY, useScheduledTasksActions } from "./use-scheduled-tasks-actions";

function buildTask(id: string, overrides: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    id,
    name: `Task ${id}`,
    content: "Do the thing",
    agentId: "onepiece",
    frequency: { kind: "daily", timeOfDay: "09:00" },
    enabled: true,
    nextRunAt: "2026-08-31T09:00:00.000Z",
    latestStatus: "never-run",
    latestRunAt: null,
    latestRunSessionId: null,
    latestError: null,
    createdAt: "2026-08-01T00:00:00.000Z",
    updatedAt: "2026-08-01T00:00:00.000Z",
    version: 1,
    ...overrides,
  };
}

describe("isScheduledTaskVersionConflict", () => {
  it("matches the stable code prefix both backends emit verbatim", () => {
    expect(isScheduledTaskVersionConflict(new Error("scheduled-task-version-conflict: expected 1, stored 2"))).toBe(true);
  });

  it("does not match an unrelated error", () => {
    expect(isScheduledTaskVersionConflict(new Error("Scheduled task was not found."))).toBe(false);
  });
});

describe("useScheduledTasksActions", () => {
  beforeAll(async () => activateAppLanguage("en"));

  beforeEach(() => {
    for (const mock of Object.values(mocks)) mock.mockReset();
    mocks.listScheduledTasks.mockResolvedValue([]);
  });

  it("loads the task list on mount", async () => {
    mocks.listScheduledTasks.mockResolvedValue([buildTask("t-a")]);
    const { result } = renderHook(() => useScheduledTasksActions());

    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.tasks).toEqual([buildTask("t-a")]);
  });

  it("create: prepends the server row and clears the create mutation slot on success", async () => {
    const created = buildTask("t-new");
    mocks.createScheduledTask.mockResolvedValueOnce(created);
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    let returned: ScheduledTask | undefined;
    await act(async () => {
      returned = await result.current.create({ agentId: "onepiece", content: "x", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "New" });
    });

    expect(returned).toEqual(created);
    expect(result.current.tasks[0]).toEqual(created);
    expect(result.current.mutations.get(SCHEDULED_TASK_CREATE_MUTATION_KEY)).toBeUndefined();
  });

  it("create: fails the create mutation slot and rethrows, without touching tasks", async () => {
    mocks.createScheduledTask.mockRejectedValueOnce(new Error("Scheduled task name is required."));
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await expect(result.current.create({ agentId: "onepiece", content: "", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "" }))
        .rejects.toThrow("Scheduled task name is required.");
    });

    expect(result.current.tasks).toEqual([]);
    expect(result.current.mutations.get(SCHEDULED_TASK_CREATE_MUTATION_KEY)?.error?.message).toBe("Scheduled task name is required.");
  });

  it("setEnabled: patches only the affected task's row", async () => {
    const taskA = buildTask("t-a", { enabled: true });
    const taskB = buildTask("t-b", { enabled: true });
    mocks.listScheduledTasks.mockResolvedValue([taskA, taskB]);
    mocks.setScheduledTaskEnabled.mockResolvedValueOnce({ ...taskA, enabled: false });
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => { await result.current.setEnabled(taskA, false); });

    expect(result.current.tasks.find((task) => task.id === "t-a")?.enabled).toBe(false);
    expect(result.current.tasks.find((task) => task.id === "t-b")?.enabled).toBe(true);
  });

  // 19.17: the core claim this hook exists to satisfy -- two different tasks' own mutations are
  // independently pending/settled, not lumped under one shared flag the way the pre-19.16 panel's
  // single `error`/`saving` state used to behave.
  it("keeps two different tasks' own mutation state fully independent while both are in flight", async () => {
    const taskA = buildTask("t-a");
    const taskB = buildTask("t-b");
    mocks.listScheduledTasks.mockResolvedValue([taskA, taskB]);
    let resolveA: (value: ScheduledTask) => void = () => {};
    mocks.setScheduledTaskEnabled.mockImplementation(() => new Promise((resolve) => { resolveA = resolve; }));
    mocks.deleteScheduledTask.mockResolvedValueOnce(undefined);
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => { void result.current.setEnabled(taskA, false); });
    expect(result.current.mutations.get("t-a")?.pending).toBe(true);
    expect(result.current.mutations.get("t-b")).toBeUndefined();

    await act(async () => { await result.current.remove(taskB); });
    expect(result.current.mutations.get("t-b")).toBeUndefined(); // succeeded and cleared
    expect(result.current.tasks.some((task) => task.id === "t-b")).toBe(false);
    // Task A's own still-pending mutation was never touched by task B's unrelated, already-settled one.
    expect(result.current.mutations.get("t-a")?.pending).toBe(true);

    await act(async () => { resolveA({ ...taskA, enabled: false }); await Promise.resolve(); });
    expect(result.current.mutations.get("t-a")).toBeUndefined();
    expect(result.current.tasks.find((task) => task.id === "t-a")?.enabled).toBe(false);
  });

  it("update: patches the task and returns the server row on success", async () => {
    const task = buildTask("t-a", { version: 3 });
    mocks.listScheduledTasks.mockResolvedValue([task]);
    const updated = { ...task, name: "Renamed", version: 4 };
    mocks.updateScheduledTask.mockResolvedValueOnce(updated);
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    let returned: ScheduledTask | undefined;
    await act(async () => {
      returned = await result.current.update(task, { agentId: "onepiece", content: "x", expectedVersion: 3, frequency: task.frequency, name: "Renamed", taskId: "t-a" });
    });

    expect(returned).toEqual(updated);
    expect(result.current.tasks[0]).toEqual(updated);
  });

  it("update: on a version conflict, fails with a translated message and rethrows the raw error for the caller to inspect", async () => {
    const task = buildTask("t-a", { version: 3 });
    mocks.listScheduledTasks.mockResolvedValue([task]);
    mocks.updateScheduledTask.mockRejectedValueOnce(new Error("scheduled-task-version-conflict: expected 3, stored 4"));
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await expect(result.current.update(task, { agentId: "onepiece", content: "x", expectedVersion: 3, frequency: task.frequency, name: "Renamed", taskId: "t-a" }))
        .rejects.toThrow("scheduled-task-version-conflict");
    });

    const failure = result.current.mutations.get("t-a");
    expect(failure?.error?.message).not.toContain("scheduled-task-version-conflict");
    expect(failure?.error?.message).toContain("changed elsewhere");
    // The row itself is left exactly as it was -- a conflict never silently overwrites what is shown.
    expect(result.current.tasks[0]).toEqual(task);
  });

  it("remove: rolls back to the original index if the delete is rejected", async () => {
    const taskA = buildTask("t-a");
    const taskB = buildTask("t-b");
    const taskC = buildTask("t-c");
    mocks.listScheduledTasks.mockResolvedValue([taskA, taskB, taskC]);
    mocks.deleteScheduledTask.mockRejectedValueOnce(new Error("Scheduled task was not found."));
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => { await result.current.remove(taskB); });

    expect(result.current.tasks.map((task) => task.id)).toEqual(["t-a", "t-b", "t-c"]);
    expect(result.current.mutations.get("t-b")?.error?.message).toBe("Scheduled task was not found.");
  });

  it("runNow: never touches tasks, only this task's own mutation slot", async () => {
    const task = buildTask("t-a");
    mocks.listScheduledTasks.mockResolvedValue([task]);
    const run = { id: "run-1", taskId: "t-a", sessionId: "session-1", status: "succeeded" as const, error: null, startedAt: "2026-08-31T09:00:00.000Z", completedAt: "2026-08-31T09:00:00.000Z" };
    mocks.runScheduledTaskNow.mockResolvedValueOnce({ run, operationId: null });
    const { result } = renderHook(() => useScheduledTasksActions());
    await waitFor(() => expect(result.current.loading).toBe(false));

    let returned;
    await act(async () => { returned = await result.current.runNow(task); });

    expect(returned).toEqual({ run, operationId: null });
    expect(result.current.tasks[0]).toEqual(task);
    expect(result.current.mutations.get("t-a")).toBeUndefined();
  });
});
