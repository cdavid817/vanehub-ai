import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CuratorCandidateState,
  CuratorNotificationEvent,
  CuratorNotificationEventKind,
  CuratorNotificationNavigationTarget,
  CuratorRisk,
  CuratorRoute,
} from "../types/skill-curator";

const eventKinds = new Set<CuratorNotificationEventKind>([
  "pending_review", "deferral_date", "supersession", "rejection", "apply_success", "apply_failure",
  "probation_regression",
]);
const states = new Set<CuratorCandidateState>([
  "pending", "awaiting_draft", "ready_for_review", "deferred", "rejected",
  "applying", "applied", "apply_failed", "superseded",
]);
const risks = new Set<CuratorRisk>(["low", "medium", "high"]);
const routes = new Set<CuratorRoute>(["advance", "needs_human_review"]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function boundedText(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 256;
}

function navigation(value: unknown): CuratorNotificationNavigationTarget | undefined {
  if (!isRecord(value) || !boundedText(value.candidateId)) return undefined;
  if (value.kind === "candidate_review" && Object.keys(value).every((key) => ["kind", "candidateId"].includes(key))) {
    return { kind: "candidate_review", candidateId: value.candidateId };
  }
  if (
    value.kind === "overlay_history"
    && boundedText(value.skillId)
    && boundedText(value.overlayHistoryId)
    && Object.keys(value).every((key) => ["kind", "candidateId", "skillId", "overlayHistoryId"].includes(key))
  ) {
    return {
      kind: "overlay_history",
      candidateId: value.candidateId,
      skillId: value.skillId,
      overlayHistoryId: value.overlayHistoryId,
    };
  }
  return undefined;
}

export function normalizeCuratorNotification(value: unknown): CuratorNotificationEvent | undefined {
  if (!isRecord(value)) return undefined;
  const target = navigation(value.navigationTarget);
  if (
    value.schemaVersion !== 1
    || typeof value.eventKind !== "string"
    || !eventKinds.has(value.eventKind as CuratorNotificationEventKind)
    || !boundedText(value.candidateId)
    || !Number.isSafeInteger(value.candidateRevision)
    || (value.candidateRevision as number) < 1
    || !boundedText(value.workspaceId)
    || !boundedText(value.skillId)
    || !boundedText(value.overlayScope)
    || typeof value.state !== "string"
    || !states.has(value.state as CuratorCandidateState)
    || typeof value.risk !== "string"
    || !risks.has(value.risk as CuratorRisk)
    || typeof value.route !== "string"
    || !routes.has(value.route as CuratorRoute)
    || target === undefined
    || !Object.keys(value).every((key) => [
      "schemaVersion", "eventKind", "candidateId", "candidateRevision", "workspaceId",
      "skillId", "overlayScope", "state", "risk", "route", "navigationTarget",
    ].includes(key))
  ) return undefined;
  return {
    schemaVersion: 1,
    eventKind: value.eventKind as CuratorNotificationEventKind,
    candidateId: value.candidateId,
    candidateRevision: value.candidateRevision as number,
    workspaceId: value.workspaceId,
    skillId: value.skillId,
    overlayScope: value.overlayScope,
    state: value.state as CuratorCandidateState,
    risk: value.risk as CuratorRisk,
    route: value.route as CuratorRoute,
    navigationTarget: target,
  };
}

export async function subscribeTauriSkillCuratorNotifications(
  handler: (event: CuratorNotificationEvent) => void,
): Promise<() => void> {
  const unlisten = await listen<unknown>("skill-curator:notification", ({ payload }) => {
    const event = normalizeCuratorNotification(payload);
    if (event) handler(event);
  });
  try {
    await invoke("dispatch_skill_curator_notifications");
  } catch {
    // Notification recovery is best effort and must not affect Curator or Agent availability.
  }
  return unlisten;
}
