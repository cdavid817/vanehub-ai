import type {
  CuratorNotificationEvent,
  CuratorNotificationEventKind,
} from "../types/skill-curator";
import { getPolicy, type WebCuratorCandidate } from "./web-skill-curator-state";

type Handler = (event: CuratorNotificationEvent) => void;

const handlers = new Set<Handler>();
const receipts = new Set<string>();

function createEvent(
  candidate: WebCuratorCandidate,
  eventKind: CuratorNotificationEventKind,
): CuratorNotificationEvent | undefined {
  const { detail } = candidate;
  if (eventKind === "apply_success") {
    const overlayHistoryId = detail.application?.overlayHistoryId;
    if (!overlayHistoryId) return undefined;
    return {
      schemaVersion: 1,
      eventKind,
      candidateId: detail.candidateId,
      candidateRevision: detail.revision,
      workspaceId: detail.workspaceId,
      skillId: detail.targetSkillId,
      overlayScope: detail.overlayScope,
      state: detail.state,
      risk: detail.risk,
      route: detail.route,
      navigationTarget: {
        kind: "overlay_history",
        candidateId: detail.candidateId,
        skillId: detail.targetSkillId,
        overlayHistoryId,
      },
    };
  }
  return {
    schemaVersion: 1,
    eventKind,
    candidateId: detail.candidateId,
    candidateRevision: detail.revision,
    workspaceId: detail.workspaceId,
    skillId: detail.targetSkillId,
    overlayScope: detail.overlayScope,
    state: detail.state,
    risk: detail.risk,
    route: detail.route,
    navigationTarget: { kind: "candidate_review", candidateId: detail.candidateId },
  };
}

export function publishWebCuratorNotification(
  candidate: WebCuratorCandidate,
  eventKind: CuratorNotificationEventKind,
): void {
  const key = `${candidate.detail.candidateId}:${candidate.detail.revision}:${eventKind}`;
  if (receipts.has(key)) return;
  receipts.add(key);
  candidate.notificationPending = false;
  if (!getPolicy(candidate.detail.workspaceId).notificationsEnabled) return;
  const event = createEvent(candidate, eventKind);
  if (!event) return;
  for (const handler of handlers) {
    try {
      handler(structuredClone(event));
    } catch {
      // A notification consumer is isolated from Curator state transitions.
    }
  }
}

export async function subscribeWebSkillCuratorNotifications(handler: Handler): Promise<() => void> {
  handlers.add(handler);
  return () => handlers.delete(handler);
}

export function resetWebSkillCuratorNotificationsForTest(): void {
  handlers.clear();
  receipts.clear();
}
