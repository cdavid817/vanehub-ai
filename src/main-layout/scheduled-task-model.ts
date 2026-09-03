import { validateScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type { ScheduledTaskFrequency } from "../types/agent";

export type FrequencyKind = ScheduledTaskFrequency["kind"];

export interface ScheduledTaskDraft {
  agentId: string;
  content: string;
  frequency: ScheduledTaskFrequency;
  name: string;
}

export function initialFrequency(kind: FrequencyKind): ScheduledTaskFrequency {
  switch (kind) {
    case "minutes":
      return { kind, interval: 30 };
    case "hours":
      return { kind, interval: 1 };
    case "daily":
      return { kind, timeOfDay: "09:00" };
    case "weekly":
      return { kind, weekday: 1, timeOfDay: "09:00" };
    case "monthly":
      return { kind, dayOfMonth: 1, timeOfDay: "09:00" };
  }
}

export function initialScheduledTaskDraft(agentId = ""): ScheduledTaskDraft {
  return { agentId, content: "", frequency: initialFrequency("daily"), name: "" };
}

export function isValidScheduledTaskFrequency(frequency: ScheduledTaskFrequency) {
  try {
    validateScheduledTaskFrequency(frequency);
    return true;
  } catch {
    return false;
  }
}

export function isValidScheduledTaskDraft(draft: ScheduledTaskDraft) {
  return Boolean(
    draft.name.trim()
    && draft.content.trim()
    && draft.agentId
    && isValidScheduledTaskFrequency(draft.frequency),
  );
}
