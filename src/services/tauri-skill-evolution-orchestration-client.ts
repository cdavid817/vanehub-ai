import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  EvolutionNotificationEvent,
  EvolutionNotificationEventKind,
  SkillEvolutionOrchestrationService,
} from "./skill-evolution-orchestration-service";

const notificationKinds = new Set<EvolutionNotificationEventKind>([
  "run_attention", "automatic_application", "probation_regression",
  "breaker_opened", "breaker_recovered",
]);

function boundedOptional(value: unknown): value is string | null {
  return value === null || (typeof value === "string" && value.length > 0 && value.length <= 512);
}

export function normalizeEvolutionNotification(value: unknown): EvolutionNotificationEvent | null {
  if (typeof value !== "object" || value === null) return null;
  const event = value as Record<string, unknown>;
  const exactKeys = [
    "schemaVersion", "eventId", "eventKind", "workspaceId", "runId", "applicationId",
    "probationId", "breakerId", "skillId", "safeReasonCode", "probationEndsAtMs", "entityRevision",
  ];
  if (
    event.schemaVersion !== 1
    || typeof event.eventId !== "string" || event.eventId.length < 1 || event.eventId.length > 512
    || typeof event.eventKind !== "string"
    || !notificationKinds.has(event.eventKind as EvolutionNotificationEventKind)
    || typeof event.workspaceId !== "string" || event.workspaceId.length < 1 || event.workspaceId.length > 256
    || !boundedOptional(event.runId) || !boundedOptional(event.applicationId)
    || !boundedOptional(event.probationId) || !boundedOptional(event.breakerId)
    || !boundedOptional(event.skillId) || !boundedOptional(event.safeReasonCode)
    || !(event.probationEndsAtMs === null || Number.isSafeInteger(event.probationEndsAtMs))
    || !Number.isSafeInteger(event.entityRevision) || (event.entityRevision as number) < 0
    || !Object.keys(event).every((key) => exactKeys.includes(key))
  ) return null;
  return event as unknown as EvolutionNotificationEvent;
}

export const tauriSkillEvolutionOrchestrationClient: SkillEvolutionOrchestrationService = {
  getEvolutionSchedulerOverview(workspaceId) {
    return invoke("get_skill_evolution_scheduler_overview", { workspaceId });
  },
  getEvolutionPolicy(workspaceId) {
    return invoke("get_skill_evolution_policy", { workspaceId });
  },
  updateEvolutionPolicy(input) {
    return invoke("update_skill_evolution_policy", { input });
  },
  listEvolutionRuns(input) {
    return invoke("list_skill_evolution_runs", { input });
  },
  getEvolutionRun(runId) {
    return invoke("get_skill_evolution_run", { runId });
  },
  listEvolutionEligibility(input) {
    return invoke("list_skill_evolution_eligibility", { input });
  },
  listEvolutionApplications(input) {
    return invoke("list_skill_evolution_applications", { input });
  },
  listEvolutionProbations(input) {
    return invoke("list_skill_evolution_probations", { input });
  },
  listEvolutionBreakers(input) {
    return invoke("list_skill_evolution_breakers", { input });
  },
  requestEvolutionRun(workspaceId) {
    return invoke("request_skill_evolution_run", { workspaceId });
  },
  cancelEvolutionRun(runId, expectedRevision) {
    return invoke("cancel_skill_evolution_run", { runId, expectedRevision });
  },
  acknowledgeEvolutionBreaker(breakerId, expectedRevision) {
    return invoke("acknowledge_skill_evolution_breaker", { breakerId, expectedRevision });
  },
  async subscribeEvolutionNotifications(handler) {
    const unlisten = await listen<unknown>("skill-evolution-orchestration:notification", ({ payload }) => {
      const event = normalizeEvolutionNotification(payload);
      if (event) handler(event);
    });
    try {
      await invoke("dispatch_skill_evolution_notifications");
    } catch {
      // Delivery recovery is isolated from orchestration and all Agent execution.
    }
    return unlisten;
  },
};
