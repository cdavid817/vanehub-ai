import type { GenerationJobDetail, GenerationNotificationEvent } from "./skill-generation-service";

const subscribers = new Set<(event: GenerationNotificationEvent) => void>();

export function resetWebGenerationNotifications(): void { subscribers.clear(); }

export function publishWebGenerationNotification(
  job: GenerationJobDetail,
  eventKind: GenerationNotificationEvent["eventKind"],
): void {
  const event: GenerationNotificationEvent = {
    schemaVersion: 1, eventId: `${eventKind}:${job.jobId}:${job.updatedAt}`, eventKind,
    jobId: job.jobId, workspaceId: job.workspaceId ?? "global", seedId: job.seedId,
    safeFailureCode: job.safeFailureCode,
  };
  subscribers.forEach((subscriber) => subscriber(structuredClone(event)));
}

export function subscribeWebGenerationNotifications(
  handler: (event: GenerationNotificationEvent) => void,
): () => void {
  subscribers.add(handler);
  return () => subscribers.delete(handler);
}
