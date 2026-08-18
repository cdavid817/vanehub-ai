import { mockAgents } from "./mock-agent-data";
import type { ScheduledTaskService } from "./scheduled-task-service";
import { nowIso } from "./web-mock-clock";
import { computeNextScheduledRun, validateScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type {
  AutomaticArchivalSettings,
  CreateScheduledTaskInput,
  ScheduledTask,
  ScheduledTaskRun,
} from "../types/agent";

let automaticArchivalSettings: AutomaticArchivalSettings = { enabled: true, inactiveDays: 10 };
let scheduledTasks: ScheduledTask[] = [];
let nextScheduledTaskId = 1;

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

  async deleteScheduledTask(taskId) {
    findScheduledTask(taskId);
    scheduledTasks = scheduledTasks.filter((task) => task.id !== taskId);
  },
};
