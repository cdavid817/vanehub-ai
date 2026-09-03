import { describe, expect, it } from "vitest";
import { webScheduledTaskClient } from "./web-scheduled-task-client";

async function createTask(name: string) {
  return webScheduledTaskClient.createScheduledTask({
    name, content: "Do the thing", agentId: "onepiece", frequency: { kind: "daily", timeOfDay: "09:00" },
  });
}

describe("web scheduled task run history (19.11)", () => {
  it("lazily seeds several rows mixing normal/backfilled/failed outcomes, ordered newest first", async () => {
    const task = await createTask("History task");
    const runs = await webScheduledTaskClient.listScheduledTaskRuns(task.id);

    expect(runs.length).toBeGreaterThanOrEqual(3);
    expect(runs.map((run) => run.status)).toEqual(expect.arrayContaining(["succeeded", "backfilled", "failed"]));
    for (let index = 1; index < runs.length; index += 1) {
      expect(new Date(runs[index - 1].startedAt).getTime()).toBeGreaterThanOrEqual(new Date(runs[index].startedAt).getTime());
    }
    // Safe failure: the failed row carries a real, non-empty error rather than a silently blank one.
    const failed = runs.find((run) => run.status === "failed");
    expect(failed?.error).toBeTruthy();
  });

  it("seeding is idempotent -- a second call returns the exact same rows, not freshly randomized ones", async () => {
    const task = await createTask("Idempotent task");
    const first = await webScheduledTaskClient.listScheduledTaskRuns(task.id);
    const second = await webScheduledTaskClient.listScheduledTaskRuns(task.id);
    expect(second.map((run) => run.id)).toEqual(first.map((run) => run.id));
  });

  // Previously a real gap (see this file's own 19.10 doc comment): runScheduledTaskNow returned a
  // receipt but never recorded it anywhere, so it was invisible to listScheduledTaskRuns entirely.
  it("records a manual run so it is genuinely visible through listScheduledTaskRuns afterwards, newest first", async () => {
    const task = await createTask("Manual run task");
    const before = await webScheduledTaskClient.listScheduledTaskRuns(task.id);

    const { run } = await webScheduledTaskClient.runScheduledTaskNow(task.id);
    const after = await webScheduledTaskClient.listScheduledTaskRuns(task.id);

    expect(after).toHaveLength(before.length + 1);
    expect(after[0].id).toBe(run.id);
    expect(after[0].status).toBe("succeeded");
  });

  // 19.10's own contract, unchanged by 19.11's recording fix: a manual run must never advance the
  // task's own recurrence/status bookkeeping.
  it("still does not change the task's own nextRunAt/latestStatus when recording a manual run", async () => {
    const task = await createTask("Untouched bookkeeping task");
    await webScheduledTaskClient.runScheduledTaskNow(task.id);
    const tasks = await webScheduledTaskClient.listScheduledTasks();
    const match = tasks.find((candidate) => candidate.id === task.id);
    expect(match?.nextRunAt).toBe(task.nextRunAt);
    expect(match?.latestStatus).toBe("never-run");
  });

  it("still throws for an unknown task id, unchanged by this pass", async () => {
    await expect(webScheduledTaskClient.listScheduledTaskRuns("no-such-task")).rejects.toThrow();
  });
});
