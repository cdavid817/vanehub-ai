import { beforeEach, describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";

describe("web evaluation runtime", () => {
  // `limit: 50` (the service's own page-size ceiling, `MAX_EVALUATION_PAGE_LIMIT`) rather than the
  // default: this file's own tests accumulate arenas across cases (the module-level store is never
  // truly emptied, only cancelled), and a default-sized page could stop covering all of them once
  // enough tests below (including 18.6's own pagination cases) have each added a few more.
  beforeEach(async () => { for (const arena of (await webAgentClient.listEvaluationArenas({ limit: 50 })).items) await webAgentClient.cancelEvaluation(arena.id); });

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

  // 18.6: real pagination over `webEvaluationArenas`, not the previous "return everything" stand-in
  // -- a page must not silently repeat or skip an item relative to the one before it.
  describe("listEvaluationArenas pagination", () => {
    it("pages newest-first with a real cursor, the second page picking up exactly where the first left off", async () => {
      const tasks = await webAgentClient.listEvaluationTasks();
      const start = () => webAgentClient.startEvaluation({ taskId: tasks[0].id, taskVersion: tasks[0].version, agentIds: ["onepiece"] });
      const a = await start(); const b = await start(); const c = await start();
      const first = await webAgentClient.listEvaluationArenas({ limit: 2 });
      expect(first.items.map((arena) => arena.id)).toEqual([c.id, b.id]);
      expect(first.nextCursor).toBe("2");
      const second = await webAgentClient.listEvaluationArenas({ cursor: first.nextCursor, limit: 2 });
      expect(second.items[0].id).toBe(a.id);
      expect(second.items.map((arena) => arena.id)).not.toEqual(expect.arrayContaining([b.id, c.id]));
    });

    it("walks the cursor to a real end (null), never looping back to page one", async () => {
      const tasks = await webAgentClient.listEvaluationTasks();
      const arena = await webAgentClient.startEvaluation({ taskId: tasks[0].id, taskVersion: tasks[0].version, agentIds: ["onepiece"] });
      const first = await webAgentClient.listEvaluationArenas({ limit: 1 });
      expect(first.items[0].id).toBe(arena.id);
      let cursor = first.nextCursor;
      let guard = 0;
      const seen = new Set<string>();
      while (cursor && guard < 50) {
        const page = await webAgentClient.listEvaluationArenas({ cursor, limit: 5 });
        for (const item of page.items) {
          expect(seen.has(item.id)).toBe(false); // a repeated id would mean a page looped back
          seen.add(item.id);
        }
        cursor = page.nextCursor;
        guard += 1;
      }
      expect(cursor).toBeNull();
    });

    it("rejects an invalid cursor rather than silently resetting to page one", async () => {
      await expect(webAgentClient.listEvaluationArenas({ cursor: "not-a-number" })).rejects.toThrow("invalid evaluation cursor");
      await expect(webAgentClient.listEvaluationArenas({ cursor: "-1" })).rejects.toThrow("invalid evaluation cursor");
    });
  });
});
