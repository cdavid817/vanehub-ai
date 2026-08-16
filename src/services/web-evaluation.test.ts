import { beforeEach, describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";

describe("web evaluation runtime", () => {
  beforeEach(async () => { for (const arena of await webAgentClient.listEvaluationArenas()) await webAgentClient.cancelEvaluation(arena.id); });

  it("runs a deterministic isolated arena and exports missing metrics honestly", async () => {
    const tasks = await webAgentClient.listEvaluationTasks();
    expect(tasks).toHaveLength(3);
    const arena = await webAgentClient.startEvaluation({ taskId: tasks[0].id, taskVersion: tasks[0].version, agentIds: ["onepiece", "codex-cli"] });
    expect(arena.attempts.map((attempt) => attempt.canonicalRunId)).toEqual(expect.arrayContaining([expect.stringContaining("run-1"), expect.stringContaining("run-2")]));
    expect(arena.attempts[0].outcome).toBe("succeeded");
    expect(arena.attempts[1].outcome).toBe("task_failed");
    expect(arena.attempts[1].metrics.find((metric) => metric.name === "input_tokens")).toMatchObject({ value: null, quality: "unavailable" });
    expect(await webAgentClient.exportEvaluation(arena.id)).toMatchObject({ schemaVersion: 1, arena: { rankingVersion: "deterministic-v1" } });
  });

  it("rejects empty and oversized arenas", async () => {
    await expect(webAgentClient.startEvaluation({ taskId: "fix-null-auth-token", taskVersion: 1, agentIds: [] })).rejects.toThrow();
    await expect(webAgentClient.startEvaluation({ taskId: "fix-null-auth-token", taskVersion: 1, agentIds: Array.from({ length: 9 }, (_, index) => `agent-${index}`) })).rejects.toThrow();
  });
});
