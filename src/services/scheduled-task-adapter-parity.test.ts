import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";
import type { ScheduledTaskFrequency } from "../types/agent";

const frequency: ScheduledTaskFrequency = { kind: "daily", timeOfDay: "09:00" };

/**
 * 19.18: unlike Goal/Loop/Token-usage/Skill-overlay (each with their own dedicated
 * `tauri-x-client.ts`, already covered by their own `x-adapter-parity.test.ts`), Scheduled Tasks'
 * Tauri methods live inside the shared, all-domain `tauri-agent-client.ts` -- confirmed by reading
 * it directly (no separate `tauri-scheduled-task-client.ts` exists, only a Web-side
 * `web-scheduled-task-client.ts`) -- so no equivalent parity file existed for this domain before
 * this pass. `web-agent-client.test.ts`/`web-scheduled-task-client.test.ts` already exercise the
 * Web mock's own business logic thoroughly; `use-scheduled-tasks-actions.test.ts`'s own
 * `isScheduledTaskVersionConflict` tests already prove the version-conflict error code is stable
 * across both backends. What was genuinely untested is the IPC boundary itself: whether
 * `tauriAgentClient`'s scheduled-task methods actually call `invoke` with the command name and
 * argument shape the Rust side expects, and whether the pass-through preserves a real payload's
 * shape unmangled -- mirroring `goal-adapter-parity.test.ts`'s own two-test shape exactly.
 */
describe("Scheduled task adapter parity (19.18)", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("passes the same argument shape across every scheduled-task command", async () => {
    invokeMock.mockResolvedValue(undefined);

    await tauriAgentClient.listScheduledTasks();
    await tauriAgentClient.listScheduledTaskRuns("task-1");
    await tauriAgentClient.listScheduledTaskRuns("task-1", { cursor: "20", limit: 10 });
    await tauriAgentClient.createScheduledTask({ agentId: "onepiece", content: "Summarize the week", frequency, name: "Weekly report" });
    await tauriAgentClient.setScheduledTaskEnabled({ enabled: false, taskId: "task-1" });
    await tauriAgentClient.updateScheduledTask({ agentId: "onepiece", content: "Summarize the week", expectedVersion: 1, frequency, name: "Weekly report v2", taskId: "task-1" });
    await tauriAgentClient.deleteScheduledTask("task-1");
    await tauriAgentClient.runScheduledTaskNow("task-1");

    expect(invokeMock.mock.calls).toEqual([
      ["list_scheduled_tasks"],
      ["list_scheduled_task_runs", { taskId: "task-1", cursor: null, limit: null }],
      ["list_scheduled_task_runs", { taskId: "task-1", cursor: "20", limit: 10 }],
      ["create_scheduled_task", { input: { agentId: "onepiece", content: "Summarize the week", frequency, name: "Weekly report" } }],
      ["set_scheduled_task_enabled", { input: { enabled: false, taskId: "task-1" } }],
      ["update_scheduled_task", { input: { agentId: "onepiece", content: "Summarize the week", expectedVersion: 1, frequency, name: "Weekly report v2", taskId: "task-1" } }],
      ["delete_scheduled_task", { taskId: "task-1" }],
      ["run_scheduled_task_now", { taskId: "task-1" }],
    ]);
  });

  it("returns an identical scheduled task contract from desktop and Web, unmangled by the pass-through", async () => {
    const web = await webAgentClient.createScheduledTask({ agentId: "onepiece", content: "Summarize the week", frequency, name: "Weekly report" });
    invokeMock.mockResolvedValueOnce(web);

    await expect(
      tauriAgentClient.createScheduledTask({ agentId: "onepiece", content: "Summarize the week", frequency, name: "Weekly report" }),
    ).resolves.toEqual(web);
  });
});
