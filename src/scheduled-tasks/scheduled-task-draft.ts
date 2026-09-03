import { validateScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type {
  AgentRegistryEntry, CreateScheduledTaskInput, ScheduledTask, ScheduledTaskFrequency, UpdateScheduledTaskInput,
} from "../types/agent";
import { initialFrequency } from "./scheduled-task-presentation";

/** The editable fields shared by Create and Edit (19.7) -- a plain, framework-agnostic shape so
 *  it can be validated and converted to service inputs without a React import, matching
 *  `scheduled-task-presentation.ts`'s own precedent. */
export interface ScheduledTaskDraft {
  name: string;
  content: string;
  agentId: string;
  frequency: ScheduledTaskFrequency;
}

/** One issue at a time, like `validateLoopDefinitionStep` (loop-definition-form.ts) -- adjacent
 *  validation only ever needs to point at the single field a reader should fix next. */
export type ScheduledTaskDraftIssue = "name" | "content" | "agent" | "frequency";

export function blankScheduledTaskDraft(defaultAgentId: string): ScheduledTaskDraft {
  return { agentId: defaultAgentId, content: "", frequency: initialFrequency("daily"), name: "" };
}

export function scheduledTaskDraftFromTask(task: ScheduledTask): ScheduledTaskDraft {
  return { agentId: task.agentId, content: task.content, frequency: { ...task.frequency }, name: task.name };
}

/** 19.9: Duplicate's own prefill -- everything copied verbatim except the name, which the caller
 *  adjusts (`scheduledTasks.copyName`, mirroring `loops.definition.copyName`) so two tasks never
 *  silently share a name. */
export function duplicateScheduledTaskDraft(source: ScheduledTask, copyName: string): ScheduledTaskDraft {
  return { ...scheduledTaskDraftFromTask(source), name: copyName };
}

export function validateScheduledTaskDraft(
  draft: ScheduledTaskDraft,
  agents: AgentRegistryEntry[],
): ScheduledTaskDraftIssue | null {
  if (!draft.name.trim()) return "name";
  if (!draft.content.trim()) return "content";
  if (!draft.agentId || !agents.some((agent) => agent.id === draft.agentId)) return "agent";
  try {
    validateScheduledTaskFrequency(draft.frequency);
  } catch {
    return "frequency";
  }
  return null;
}

export function toCreateScheduledTaskInput(draft: ScheduledTaskDraft): CreateScheduledTaskInput {
  return { agentId: draft.agentId, content: draft.content.trim(), frequency: draft.frequency, name: draft.name.trim() };
}

export function toUpdateScheduledTaskInput(task: ScheduledTask, draft: ScheduledTaskDraft): UpdateScheduledTaskInput {
  return {
    agentId: draft.agentId,
    content: draft.content.trim(),
    expectedVersion: task.version,
    frequency: draft.frequency,
    name: draft.name.trim(),
    taskId: task.id,
  };
}
