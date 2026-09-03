import { describe, expect, it } from "vitest";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import {
  blankScheduledTaskDraft, duplicateScheduledTaskDraft, scheduledTaskDraftFromTask, toCreateScheduledTaskInput,
  toUpdateScheduledTaskInput, validateScheduledTaskDraft,
} from "./scheduled-task-draft";

const agents: AgentRegistryEntry[] = [
  { id: "onepiece", displayName: "OnePiece", supportedInteractionModes: ["cli"] } as AgentRegistryEntry,
  { id: "claude-code", displayName: "Claude Code", supportedInteractionModes: ["cli"] } as AgentRegistryEntry,
];

function buildTask(overrides: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    id: "task-1",
    name: "Nightly digest",
    content: "Summarize today's commits",
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
    version: 3,
    ...overrides,
  };
}

describe("blankScheduledTaskDraft", () => {
  it("starts empty with the given default agent and a daily frequency", () => {
    expect(blankScheduledTaskDraft("onepiece")).toEqual({
      agentId: "onepiece", content: "", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "",
    });
  });
});

describe("scheduledTaskDraftFromTask", () => {
  it("copies the task's own editable fields, not its id/version/status", () => {
    const task = buildTask();
    expect(scheduledTaskDraftFromTask(task)).toEqual({
      agentId: "onepiece", content: "Summarize today's commits", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "Nightly digest",
    });
  });

  it("clones the frequency object so editing the draft cannot mutate the source task", () => {
    const task = buildTask();
    const draft = scheduledTaskDraftFromTask(task);
    if (draft.frequency.kind === "daily") draft.frequency.timeOfDay = "23:00";
    expect(task.frequency).toEqual({ kind: "daily", timeOfDay: "09:00" });
  });
});

describe("duplicateScheduledTaskDraft", () => {
  it("copies every field except the name, which the caller supplies", () => {
    const task = buildTask();
    expect(duplicateScheduledTaskDraft(task, "Nightly digest copy")).toEqual({
      agentId: "onepiece", content: "Summarize today's commits", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "Nightly digest copy",
    });
  });
});

describe("validateScheduledTaskDraft", () => {
  const valid = blankScheduledTaskDraft("onepiece");
  valid.name = "Task";
  valid.content = "Do it";

  it("reports no issue for a fully valid draft", () => {
    expect(validateScheduledTaskDraft(valid, agents)).toBeNull();
  });

  it("reports name when blank or whitespace-only", () => {
    expect(validateScheduledTaskDraft({ ...valid, name: "" }, agents)).toBe("name");
    expect(validateScheduledTaskDraft({ ...valid, name: "   " }, agents)).toBe("name");
  });

  it("reports content when blank", () => {
    expect(validateScheduledTaskDraft({ ...valid, content: "" }, agents)).toBe("content");
  });

  it("reports agent when unset or not in the selectable list", () => {
    expect(validateScheduledTaskDraft({ ...valid, agentId: "" }, agents)).toBe("agent");
    expect(validateScheduledTaskDraft({ ...valid, agentId: "unknown-agent" }, agents)).toBe("agent");
  });

  it("reports frequency when the recurrence fields are structurally invalid", () => {
    expect(validateScheduledTaskDraft({ ...valid, frequency: { kind: "monthly", dayOfMonth: 0, timeOfDay: "09:00" } }, agents)).toBe("frequency");
  });

  it("checks fields in a stable order so only one issue surfaces at a time", () => {
    // name wins over every other simultaneous problem, matching `ScheduledTaskForm`'s own
    // "one issue at a time" adjacent-validation contract.
    expect(validateScheduledTaskDraft({ agentId: "", content: "", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "" }, agents)).toBe("name");
  });
});

describe("toCreateScheduledTaskInput / toUpdateScheduledTaskInput", () => {
  it("trims name and content for create", () => {
    const draft = { agentId: "onepiece", content: "  do it  ", frequency: { kind: "daily" as const, timeOfDay: "09:00" }, name: "  Task  " };
    expect(toCreateScheduledTaskInput(draft)).toEqual({
      agentId: "onepiece", content: "do it", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "Task",
    });
  });

  it("carries the task's id and version as expectedVersion for update", () => {
    const task = buildTask({ version: 7 });
    const draft = scheduledTaskDraftFromTask(task);
    draft.name = "  Renamed  ";
    expect(toUpdateScheduledTaskInput(task, draft)).toEqual({
      agentId: "onepiece", content: "Summarize today's commits", expectedVersion: 7,
      frequency: { kind: "daily", timeOfDay: "09:00" }, name: "Renamed", taskId: "task-1",
    });
  });
});
