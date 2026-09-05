import type { SkillCuratorService } from "../services/skill-curator-service";
import type { CuratorCandidateSummary, CuratorQueueQuery } from "../types/skill-curator";
import { webSkillCuratorActions } from "./web-skill-curator-actions";
import {
  resetWebSkillCuratorNotificationsForTest,
  subscribeWebSkillCuratorNotifications,
} from "./web-skill-curator-notifications";
import {
  ensureWorkspace,
  failure,
  findCandidate,
  getPolicy,
  resetWebSkillCuratorForTest as resetWebStateForTest,
  success,
  type WebCuratorCandidate,
} from "./web-skill-curator-state";

const priorities = [
  "ready_for_review", "apply_failed", "awaiting_draft", "pending", "deferred",
  "applying", "rejected", "applied", "superseded",
];

function matches(candidate: WebCuratorCandidate, input: CuratorQueueQuery): boolean {
  const { detail } = candidate;
  return (input.skillId === undefined || detail.targetSkillId === input.skillId)
    && (input.states === undefined || input.states.includes(detail.state))
    && (input.routes === undefined || input.routes.includes(detail.route))
    && (input.risks === undefined || input.risks.includes(detail.risk))
    && (input.draftReady === undefined || detail.draftReady === input.draftReady)
    && (input.stale === undefined || (detail.staleness.length > 0) === input.stale)
    && (input.notificationPending === undefined || candidate.notificationPending === input.notificationPending)
    && (input.updatedBeforeMs === undefined || detail.updatedAtMs < input.updatedBeforeMs);
}

function summary(candidate: WebCuratorCandidate): CuratorCandidateSummary {
  const { candidateId, targetSkillId, state, route, risk, draftReady, staleness, revision, updatedAtMs } = candidate.detail;
  return { candidateId, targetSkillId, state, route, risk, draftReady, staleness: [...staleness], revision, updatedAtMs };
}

function cursor(value?: string): number | undefined {
  if (value === undefined) return 0;
  if (!/^\d+$/.test(value)) return undefined;
  return Number.parseInt(value, 10);
}

export function resetWebSkillCuratorForTest(): void {
  resetWebStateForTest();
  resetWebSkillCuratorNotificationsForTest();
}

export const webSkillCuratorClient: SkillCuratorService = {
  async querySkillCuratorQueue(input) {
    if (!input.workspaceId.trim()) return failure("invalid_input", "workspace_required");
    const offset = cursor(input.cursor);
    const limit = input.limit ?? 20;
    if (offset === undefined || limit < 1 || limit > 100) return failure("invalid_input", "invalid_pagination");
    const all = ensureWorkspace(input.workspaceId)
      .filter((candidate) => matches(candidate, input))
      .sort((left, right) => priorities.indexOf(left.detail.state) - priorities.indexOf(right.detail.state)
        || ({ high: 0, medium: 1, low: 2 }[left.detail.risk] - { high: 0, medium: 1, low: 2 }[right.detail.risk])
        || left.detail.updatedAtMs - right.detail.updatedAtMs
        || left.detail.candidateId.localeCompare(right.detail.candidateId));
    const items = all.slice(offset, offset + limit).map(summary);
    const nextOffset = offset + items.length;
    return success({
      items,
      ...(nextOffset < all.length ? { nextCursor: String(nextOffset) } : {}),
      totalCount: all.length,
      complete: nextOffset >= all.length,
    });
  },

  async getSkillCuratorCandidate(candidateId) {
    const candidate = findCandidate(candidateId);
    return candidate ? success(candidate.detail) : failure("not_found", "not_found");
  },

  async querySkillCuratorAudit(candidateId, nextCursor) {
    const candidate = findCandidate(candidateId);
    if (!candidate) return failure("not_found", "not_found");
    const offset = cursor(nextCursor);
    if (offset === undefined) return failure("invalid_input", "invalid_pagination", candidate);
    const limit = 20;
    const items = candidate.audit.slice(offset, offset + limit);
    const nextOffset = offset + items.length;
    return success({
      items,
      ...(nextOffset < candidate.audit.length ? { nextCursor: String(nextOffset) } : {}),
      complete: nextOffset >= candidate.audit.length,
    });
  },

  async getSkillCuratorPolicy(workspaceId) {
    if (!workspaceId.trim()) return failure("invalid_input", "workspace_required");
    return success(getPolicy(workspaceId));
  },

  ...webSkillCuratorActions,
  subscribeSkillCuratorNotifications: subscribeWebSkillCuratorNotifications,
};
