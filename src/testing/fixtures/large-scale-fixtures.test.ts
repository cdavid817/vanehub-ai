import { describe, expect, it } from "vitest";
import type { AgentRunState } from "../../types/agent-run";
import type { ScheduledTaskLatestStatus } from "../../types/agent";
import type { LoopRunStatus } from "../../types/loop";
import { FIXTURE_COUNTS, generateLargeScaleFixtures } from "./large-scale-fixtures";

describe("large-scale workbench fixtures (redesign-unified-workbench-ui task 0.9)", () => {
  const fixtures = generateLargeScaleFixtures();

  it("produces the exact counts task 0.9 asks for", () => {
    expect(fixtures.sessions).toHaveLength(FIXTURE_COUNTS.sessions);
    expect(fixtures.messages).toHaveLength(FIXTURE_COUNTS.messages);
    expect(fixtures.missionControlRuns).toHaveLength(FIXTURE_COUNTS.missionControlRuns);
    expect(fixtures.workItems).toHaveLength(FIXTURE_COUNTS.workItems);
    expect(fixtures.goals).toHaveLength(FIXTURE_COUNTS.goals);
    expect(fixtures.loopRuns).toHaveLength(FIXTURE_COUNTS.loopRuns);
    expect(fixtures.scheduledTasks).toHaveLength(FIXTURE_COUNTS.scheduledTasks);
    expect(fixtures.evaluationResultRows).toHaveLength(FIXTURE_COUNTS.evaluationResultRows);
  });

  it("gives every entity in every domain a unique id", () => {
    const idSetOf = (items: ReadonlyArray<{ id: string }>) => new Set(items.map((item) => item.id));
    expect(idSetOf(fixtures.sessions).size).toBe(fixtures.sessions.length);
    expect(idSetOf(fixtures.workItems).size).toBe(fixtures.workItems.length);
    expect(idSetOf(fixtures.goals).size).toBe(fixtures.goals.length);
    expect(idSetOf(fixtures.loopRuns).size).toBe(fixtures.loopRuns.length);
    expect(idSetOf(fixtures.scheduledTasks).size).toBe(fixtures.scheduledTasks.length);
    expect(idSetOf(fixtures.evaluationArenas).size).toBe(fixtures.evaluationArenas.length);
    expect(new Set(fixtures.messages.map((message) => message.id)).size).toBe(fixtures.messages.length);
    expect(new Set(fixtures.missionControlRuns.map((run) => run.runId)).size).toBe(fixtures.missionControlRuns.length);

    const allAttempts = fixtures.evaluationArenas.flatMap((arena) => arena.attempts);
    expect(new Set(allAttempts.map((attempt) => attempt.id)).size).toBe(allAttempts.length);
  });

  it("references only sessions that exist", () => {
    const sessionIds = new Set(fixtures.sessions.map((session) => session.id));
    for (const message of fixtures.messages) {
      expect(sessionIds.has(message.sessionId)).toBe(true);
    }
  });

  it("shapes messages per session unevenly, not uniformly, and totals exactly 5,000", () => {
    const counts = new Map<string, number>();
    for (const message of fixtures.messages) counts.set(message.sessionId, (counts.get(message.sessionId) ?? 0) + 1);
    expect([...counts.values()].reduce((sum, value) => sum + value, 0)).toBe(FIXTURE_COUNTS.messages);
    expect(counts.size).toBeLessThan(fixtures.sessions.length); // some sessions never chatted
    expect(Math.max(...counts.values())).toBeGreaterThan(50); // some sessions ran a long conversation
    expect(Math.min(...counts.values())).toBeLessThanOrEqual(3); // some sessions barely chatted
  });

  it("covers the run/task states an attention-first UI needs to render", () => {
    const runStates = new Set(fixtures.missionControlRuns.map((run) => run.state));
    const expectedRunStates: AgentRunState[] = ["failed", "blocked", "waiting_approval", "running", "completed", "cancelled"];
    for (const state of expectedRunStates) expect(runStates.has(state)).toBe(true);

    const loopStatuses = new Set(fixtures.loopRuns.map((run) => run.status));
    const expectedLoopStatuses: LoopRunStatus[] = ["succeeded", "failed", "cancelled", "running"];
    for (const status of expectedLoopStatuses) expect(loopStatuses.has(status)).toBe(true);

    const taskStatuses = new Set(fixtures.scheduledTasks.map((task) => task.latestStatus));
    const expectedTaskStatuses: ScheduledTaskLatestStatus[] = ["succeeded", "failed", "running"];
    for (const status of expectedTaskStatuses) expect(taskStatuses.has(status)).toBe(true);
  });

  it("keeps every goal's derived counts consistent with its own links", () => {
    for (const goal of fixtures.goals) {
      const counted = goal.links.filter((link) => link.targetKind !== "session" && link.progress !== "unresolvable").length;
      const terminal = goal.links.filter((link) => link.progress === "terminal").length;
      const unresolvable = goal.links.filter((link) => link.progress === "unresolvable").length;
      expect(goal.counted).toBe(counted);
      expect(goal.terminal).toBe(terminal);
      expect(goal.unresolvable).toBe(unresolvable);
    }
  });

  it("references only arenas and attempts that exist, from every evaluation result row", () => {
    const arenaIds = new Set(fixtures.evaluationArenas.map((arena) => arena.id));
    const attemptIds = new Set(fixtures.evaluationArenas.flatMap((arena) => arena.attempts.map((attempt) => attempt.id)));
    for (const row of fixtures.evaluationResultRows) {
      expect(arenaIds.has(row.arenaId)).toBe(true);
      expect(attemptIds.has(row.attemptId)).toBe(true);
    }
  });

  it("stresses truncation with some very long generated text", () => {
    expect(fixtures.sessions.some((session) => session.title.length > 150)).toBe(true);
    expect(fixtures.workItems.some((item) => item.title.length > 150)).toBe(true);
    expect(fixtures.goals.some((goal) => goal.title.length > 150)).toBe(true);
  });

  it("is deterministic: the same seed reproduces byte-identical output", () => {
    const again = generateLargeScaleFixtures();
    expect(again).toEqual(fixtures);
    expect(JSON.stringify(again)).toBe(JSON.stringify(fixtures));
  });
});
