import { mockAgents } from "./mock-agent-data";
import type { ScheduledTaskService } from "./scheduled-task-service";
import { nowIso } from "./web-mock-clock";
import { computeNextScheduledRun, sameScheduledTaskFrequency, validateScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type {
  AutomaticArchivalSettings,
  CreateScheduledTaskInput,
  ScheduledTask,
  ScheduledTaskRun,
} from "../types/agent";

let automaticArchivalSettings: AutomaticArchivalSettings = { enabled: true, inactiveDays: 10 };
let scheduledTasks: ScheduledTask[] = [];
let nextScheduledTaskId = 1;
let nextScheduledTaskRunId = 1;

function findScheduledTask(taskId: string) {
  const task = scheduledTasks.find((candidate) => candidate.id === taskId);
  if (!task) throw new Error(`Scheduled task not found: ${taskId}`);
  return task;
}

function cloneScheduledTask(task: ScheduledTask): ScheduledTask {
  return { ...task, frequency: { ...task.frequency } };
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
  async listScheduledTaskRuns(taskId) {
    const task = findScheduledTask(taskId);
    if (!task.latestRunAt) return [];
    return [{ id: `scheduled-run:${task.id}:${task.latestRunAt}`, taskId: task.id, sessionId: task.latestRunSessionId, status: task.latestStatus, error: task.latestError, startedAt: task.latestRunAt, completedAt: task.latestRunAt }] satisfies ScheduledTaskRun[];
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
    return { run, operationId: null };
  },
};
