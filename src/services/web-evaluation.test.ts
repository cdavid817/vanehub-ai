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
    expect(await webAgentClient.exportEvaluation(arena.id)).toMatchObject({ schemaVersion: 1, arena: { rankingVersion: "deterministic-v2" } });
  });

  // The desktop client records a bounded reason on an attempt whose Agent could not be dispatched
  // and no metrics at all; a mock that skipped that branch would let the two adapters drift.
  it("reports a dispatch failure with a bounded reason and no invented metrics", async () => {
    const tasks = await webAgentClient.listEvaluationTasks();
    const arena = await webAgentClient.startEvaluation({
      taskId: tasks[0].id, taskVersion: tasks[0].version, agentIds: ["onepiece", "codex-cli", "gemini-cli"],
    });
    const failed = arena.attempts[2];
    expect(failed.outcome).toBe("agent_failed");
    expect(failed.metrics).toEqual([]);
    expect(failed.checks).toEqual([{ checkId: "agent-dispatch", passed: false, summary: "evaluation Agent is not installed and available" }]);
  });

  it("rejects empty and oversized arenas", async () => {
    await expect(webAgentClient.startEvaluation({ taskId: "fix-null-auth-token", taskVersion: 1, agentIds: [] })).rejects.toThrow();
    await expect(webAgentClient.startEvaluation({ taskId: "fix-null-auth-token", taskVersion: 1, agentIds: Array.from({ length: 9 }, (_, index) => `agent-${index}`) })).rejects.toThrow();
  });
});
