import { mockAgents } from "./mock-agent-data";
import type { ScheduledTaskService } from "./scheduled-task-service";
import { daysAgoIso, nowIso } from "./web-mock-clock";
import { computeNextScheduledRun, sameScheduledTaskFrequency, validateScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type {
  AutomaticArchivalSettings,
  CreateScheduledTaskInput,
  ScheduledTask,
  ScheduledTaskRun,
  ScheduledTaskRunPage,
  ScheduledTaskRunQuery,
} from "../types/agent";

// 19.11: mirrors `web-evaluation-client.ts`'s own `DEFAULT_EVALUATION_PAGE_LIMIT`/
// `MAX_EVALUATION_PAGE_LIMIT` (20/50) -- same cursor-is-really-just-the-offset shape as the real
// Tauri command's own `DEFAULT_RUN_HISTORY_LIMIT`/`MAX_RUN_HISTORY_LIMIT`
// (`commands/sessions/scheduled_tasks.rs`), so the Web mock's pagination behavior matches what the
// desktop client actually returns instead of silently diverging.
const DEFAULT_SCHEDULED_TASK_RUN_PAGE_LIMIT = 20;
const MAX_SCHEDULED_TASK_RUN_PAGE_LIMIT = 50;

let automaticArchivalSettings: AutomaticArchivalSettings = { enabled: true, inactiveDays: 10 };
let scheduledTasks: ScheduledTask[] = [];
let nextScheduledTaskId = 1;
let nextScheduledTaskRunId = 1;
// 19.11: keyed by task id, deliberately independent of `scheduledTasks[].latestStatus` -- this
// mock never advances that field itself (there is no simulated due-task sweep here, only real
// interaction through `runScheduledTaskNow`, matching the real backend's own contract that a
// manual run must not touch it either). `ensureRunHistory` below backfills a small, plausible
// history the first time either `listScheduledTaskRuns` or `runScheduledTaskNow` is asked about a
// task that has none recorded yet, mirroring `ensureWebPromptHookVersion`'s own "build on first
// access if missing" precedent (`web-prompt-hook-versions.ts`) rather than a static seed array.
let scheduledTaskRuns: Record<string, ScheduledTaskRun[]> = {};

function findScheduledTask(taskId: string) {
  const task = scheduledTasks.find((candidate) => candidate.id === taskId);
  if (!task) throw new Error(`Scheduled task not found: ${taskId}`);
  return task;
}

function cloneScheduledTask(task: ScheduledTask): ScheduledTask {
  return { ...task, frequency: { ...task.frequency } };
}

/**
 * Plausible, deterministic (never `Math.random()`, so a rerun of the same test sees the same
 * data) synthesized history covering the real `scheduled_task_runs` status vocabulary this mock
 * previously never exercised at all: a normal success, a startup catch-up (`backfilled` -- see
 * `mark_task_succeeded`'s own `CASE status WHEN 'backfill_running' THEN 'backfilled' ELSE
 * 'succeeded' END`), and a failure with a realistic-looking error. Ordered newest-first, matching
 * `list_scheduled_task_runs`'s own `ORDER BY started_at DESC, id DESC`.
 *
 * Disclosed simplification: these rows are dated relative to real wall-clock "now" via
 * `daysAgoIso`, not derived from the owning task's own `createdAt` or `latestStatus` -- a task
 * created seconds ago in this same session can still show seeded history "from days ago." Real
 * internal consistency here would need a simulated due-task sweep (an in-browser reimplementation
 * of `bootstrap/scheduled_tasks.rs`'s own startup+60s-tick loop), which is out of scope for this
 * pass; this mirrors `mockEvaluation`'s own established precedent (`web-prompt-hook-versions.ts`)
 * of synthesizing plausible-but-not-cross-field-derived numbers for the Web/demo adapter.
 */
function seedRunHistory(task: ScheduledTask): ScheduledTaskRun[] {
  const seeds: Array<{ daysAgo: number; error: string | null; sessionId: string | null; status: ScheduledTaskRun["status"] }> = [
    { daysAgo: 2, error: null, sessionId: `web-scheduled-run-session-${task.id}-seed-succeeded`, status: "succeeded" },
    { daysAgo: 4, error: "Agent did not respond before the session timed out.", sessionId: null, status: "failed" },
    { daysAgo: 6, error: null, sessionId: `web-scheduled-run-session-${task.id}-seed-backfilled`, status: "backfilled" },
  ];
  return seeds.map((seed, index) => ({
    id: `web-scheduled-run-seed-${task.id}-${index}`,
    taskId: task.id,
    sessionId: seed.sessionId,
    status: seed.status,
    error: seed.error,
    startedAt: daysAgoIso(seed.daysAgo),
    completedAt: daysAgoIso(seed.daysAgo),
  }));
}

function ensureRunHistory(task: ScheduledTask): ScheduledTaskRun[] {
  const existing = scheduledTaskRuns[task.id];
  if (existing) return existing;
  const seeded = seedRunHistory(task);
  scheduledTaskRuns = { ...scheduledTaskRuns, [task.id]: seeded };
  return seeded;
}

/**
 * 19.11: real slicing pagination over the already newest-first `runs` array, mirroring
 * `web-evaluation-client.ts`'s own `pageEvaluationArenas` -- the cursor is genuinely just the
 * offset re-exposed as an opaque string (never hand-typed by a reader, only ever round-tripped
 * from a `nextCursor` this same client issued), so an invalid one is a real bug and throws rather
 * than silently resetting to page one, matching that same precedent's choice.
 */
function pageScheduledTaskRuns(runs: ScheduledTaskRun[], query: ScheduledTaskRunQuery | undefined): ScheduledTaskRunPage {
  const limit = Math.max(1, Math.min(query?.limit ?? DEFAULT_SCHEDULED_TASK_RUN_PAGE_LIMIT, MAX_SCHEDULED_TASK_RUN_PAGE_LIMIT));
  const offset = query?.cursor ? Number(query.cursor) : 0;
  if (!Number.isSafeInteger(offset) || offset < 0) throw new Error("invalid scheduled task run cursor");
  return {
    items: runs.slice(offset, offset + limit).map((run) => ({ ...run })),
    nextCursor: offset + limit < runs.length ? String(offset + limit) : null,
  };
}

function validateScheduledTaskInput(input: CreateScheduledTaskInput) {
  const name = input.name.trim();
  const content = input.content.trim();
  if (!name) throw new Error("Scheduled task name is required.");
  if (!content) throw new Error("Scheduled task content is required.");
  const agent = mockAgents.find((candidate) => candidate.id === input.agentId);
  if (!agent || (agent.id !== "onepiece" && !agent.supportedInteractionModes.includes("cli"))) {
    throw new Error(`Unsupported Agent: ${input.agentId}`);
  }
  validateScheduledTaskFrequency(input.frequency);
  return { name, content };
}

export const webScheduledTaskClient: ScheduledTaskService = {
  async getAutomaticArchivalSettings() {
    return { ...automaticArchivalSettings };
  },

  async saveAutomaticArchivalSettings(input) {
    if (input.inactiveDays < 1 || input.inactiveDays > 3650) {
      throw new Error("Invalid automatic archival threshold.");
    }
    automaticArchivalSettings = { ...input };
    return { ...automaticArchivalSettings };
  },

  async listScheduledTasks() {
    return scheduledTasks.map(cloneScheduledTask).sort((left, right) => left.nextRunAt.localeCompare(right.nextRunAt));
  },
  async listScheduledTaskRuns(taskId, query) {
    const task = findScheduledTask(taskId);
    return pageScheduledTaskRuns(ensureRunHistory(task), query);
  },

  async createScheduledTask(input) {
    const { name, content } = validateScheduledTaskInput(input);
    const timestamp = nowIso();
    const task: ScheduledTask = {
      id: `web-scheduled-task-${nextScheduledTaskId++}`,
      name,
      content,
      agentId: input.agentId,
      frequency: { ...input.frequency },
      enabled: true,
      nextRunAt: computeNextScheduledRun(input.frequency),
      latestStatus: "never-run",
      latestRunAt: null,
      latestRunSessionId: null,
      latestError: null,
      createdAt: timestamp,
      updatedAt: timestamp,
      version: 1,
    };
    scheduledTasks = [task, ...scheduledTasks];
    return cloneScheduledTask(task);
  },

  async setScheduledTaskEnabled(input) {
    const task = findScheduledTask(input.taskId);
    const timestamp = nowIso();
    const updated: ScheduledTask = {
      ...task,
      enabled: input.enabled,
      nextRunAt: input.enabled ? computeNextScheduledRun(task.frequency) : task.nextRunAt,
      updatedAt: timestamp,
    };
    scheduledTasks = scheduledTasks.map((candidate) => (candidate.id === input.taskId ? updated : candidate));
    return cloneScheduledTask(updated);
  },

  // 19.8: the message is the Tauri command's own contract verbatim (see
  // `scheduled_tasks::version_conflict`) -- a stable `<code>: expected X, stored Y` string rather
  // than prose, matching `personalization-revision-conflict`'s own precedent
  // (`web-personalization-rules.ts`'s `conflict()`), because that is what `CommandError` actually
  // sends across the real Tauri boundary and a friendlier mock message would leave this branch
  // untested against what the desktop really returns.
  async updateScheduledTask(input) {
    const task = findScheduledTask(input.taskId);
    if (input.expectedVersion !== task.version) {
      throw new Error(`scheduled-task-version-conflict: expected ${input.expectedVersion}, stored ${task.version}`);
    }
    const { name, content } = validateScheduledTaskInput(input);
    const timestamp = nowIso();
    // Only a real frequency change earns a fresh nextRunAt -- see sameScheduledTaskFrequency's own
    // doc comment; recomputing unconditionally would silently reschedule a task whose edit never
    // touched its frequency at all.
    const nextRunAt = sameScheduledTaskFrequency(task.frequency, input.frequency)
      ? task.nextRunAt
      : computeNextScheduledRun(input.frequency);
    const updated: ScheduledTask = {
      ...task,
      name,
      content,
      agentId: input.agentId,
      frequency: { ...input.frequency },
      nextRunAt,
      version: task.version + 1,
      updatedAt: timestamp,
    };
    scheduledTasks = scheduledTasks.map((candidate) => (candidate.id === input.taskId ? updated : candidate));
    return cloneScheduledTask(updated);
  },

  async deleteScheduledTask(taskId) {
    findScheduledTask(taskId);
    scheduledTasks = scheduledTasks.filter((task) => task.id !== taskId);
  },

  // 19.10: a plausible dispatch receipt, the same way every other mock in this file synthesizes
  // realistic ids/timestamps rather than deferring to Tauri. Deliberately does not touch
  // `scheduledTasks` -- the task's own `nextRunAt`/`latestStatus`/etc. stay exactly as they were,
  // matching the Tauri command's own "does not change recurrence" contract.
  //
  // 19.11: unlike before, the resulting run is now actually recorded into `scheduledTaskRuns`
  // (prepended, since it is always the newest) rather than returned and immediately forgotten --
  // previously a manual run here was invisible to `listScheduledTaskRuns` entirely, since that
  // method only ever read from `latestRunAt`, which this call never touches. This mirrors
  // `record_manual_run`'s own real contract: a durable, already-complete row, recorded, not just
  // receipted.
  async runScheduledTaskNow(taskId) {
    const task = findScheduledTask(taskId);
    const timestamp = nowIso();
    const runId = nextScheduledTaskRunId++;
    const sessionId = `web-scheduled-run-session-${runId}`;
    const run: ScheduledTaskRun = {
      id: `scheduled-run-${task.id}-${runId}`,
      taskId: task.id,
      sessionId,
      status: "succeeded",
      error: null,
      startedAt: timestamp,
      completedAt: timestamp,
    };
    scheduledTaskRuns = { ...scheduledTaskRuns, [task.id]: [run, ...ensureRunHistory(task)] };
    return { run, operationId: null };
  },
};
