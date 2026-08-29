import type { SkillCuratorService } from "../services/skill-curator-service";
import type {
  CuratorActionReceipt,
  CuratorApplicationResult,
  CuratorDiffProjection,
  CuratorDraftRevision,
  CuratorPreview,
  CuratorResult,
  CuratorVersionedAction,
} from "../types/skill-curator";
import { validateWebDraft, webDraftText } from "./web-skill-curator-draft";
import { publishWebCuratorNotification } from "./web-skill-curator-notifications";
import {
  appendAudit,
  ensureWorkspace,
  failure,
  findCandidate,
  getPolicy,
  safeState,
  setPolicy,
  stableHash,
  success,
  transition,
  type WebCuratorCandidate,
} from "./web-skill-curator-state";

type CuratorActions = Pick<SkillCuratorService, | "updateSkillCuratorPolicy"
  | "saveSkillCuratorDraft" | "previewSkillCuratorCandidate"
  | "approveSkillCuratorCandidate" | "rejectSkillCuratorCandidate" | "deferSkillCuratorCandidate"
  | "resumeSkillCuratorCandidate" | "retrySkillCuratorApplication">;
const policyKeys = new Set([
  "enqueueRoutes", "requireRejectionReason", "requireDeferReason", "maximumDeferDays",
  "openRetentionDays", "terminalRetentionDays", "notificationsEnabled", "digestEnabled",
  "draftDisplayLimitBytes", "diffDisplayLimitBytes",
]);

function requireCandidate(input: CuratorVersionedAction): WebCuratorCandidate | CuratorResult<never> {
  const candidate = findCandidate(input.candidateId);
  if (!candidate) return failure<never>("not_found", "not_found");
  if (!input.idempotencyKey.trim() || input.idempotencyKey.length > 160) {
    return failure("invalid_input", "invalid_idempotency_key", candidate);
  }
  if (candidate.detail.revision !== input.expectedCandidateRevision) {
    return failure("stale_conflict", "candidate_revision_conflict", candidate, "candidate_revision");
  }
  return candidate;
}

function isCandidate(value: unknown): value is WebCuratorCandidate {
  return typeof value === "object" && value !== null && "detail" in value;
}

function receipt(candidate: WebCuratorCandidate, key: string, duplicate = false): CuratorActionReceipt {
  return { ...safeState(candidate), actionId: key, duplicate };
}

function diff(content: string, limit: number): CuratorDiffProjection {
  const clipped = content.slice(0, limit);
  const truncated = clipped.length < content.length;
  return {
    fromHash: stableHash("web-current"),
    toHash: stableHash(content),
    addedCharacters: content.length,
    removedCharacters: 0,
    hunks: [{
      label: "web-mock-effective-guidance",
      before: { content: "", totalCharacters: 0, truncated: false },
      after: { content: clipped, totalCharacters: content.length, truncated },
    }],
    ...(truncated ? { nextCursor: String(clipped.length) } : {}),
    complete: !truncated,
  };
}

function createPreview(candidate: WebCuratorCandidate, input: Parameters<SkillCuratorService["previewSkillCuratorCandidate"]>[0]): CuratorPreview {
  const draft = candidate.detail.drafts[0];
  const content = draft.mutation.kind === "learned_guidance"
    ? draft.mutation.guidance
    : draft.mutation.newString;
  const now = Date.now();
  const effectiveDiffHash = stableHash(`${candidate.detail.witnessHash}:${draft.bodyHash}:${content}`);
  const projection = diff(content, getPolicy(candidate.detail.workspaceId).diffDisplayLimitBytes);
  return {
    previewId: `web-preview-${stableHash(`${candidate.detail.candidateId}:${input.idempotencyKey}`)}`,
    candidateId: candidate.detail.candidateId,
    candidateRevision: candidate.detail.revision,
    draftRevision: draft.revision,
    assessmentId: candidate.detail.assessmentAttemptId,
    witnessHash: stableHash(`${candidate.detail.witnessHash}:${effectiveDiffHash}`),
    effectiveDiffHash,
    diffs: { baseToCurrent: diff("", 1), currentToProposed: projection, baseToProposed: projection },
    validation: {
      scanPassed: true,
      canCommit: true,
      pinned: false,
      trusted: true,
      conflictCount: 0,
      conflictsComplete: true,
      safeRuleIds: ["web-mock-safe"],
      rulesComplete: true,
    },
    issuedAtMs: now,
    expiresAtMs: now + 15 * 60_000,
  };
}

function cachedReceipt(candidate: WebCuratorCandidate, key: string): CuratorResult<CuratorActionReceipt> | undefined {
  const existing = candidate.actionReceipts.get(key);
  return existing ? success({ ...existing, duplicate: true }) : undefined;
}

export const webSkillCuratorActions: CuratorActions = {
  async updateSkillCuratorPolicy(input) {
    ensureWorkspace(input.workspaceId);
    const current = getPolicy(input.workspaceId);
    if (input.expectedRevision !== current.revision) return failure("stale_conflict", "policy_revision_conflict");
    const { maximumDeferDays, openRetentionDays, terminalRetentionDays, draftDisplayLimitBytes, diffDisplayLimitBytes } = input.policy;
    const routesLocked = input.policy.enqueueRoutes.length === 2
      && input.policy.enqueueRoutes.includes("advance") && input.policy.enqueueRoutes.includes("needs_human_review");
    if (Object.keys(input.policy).some((key) => !policyKeys.has(key)) || !routesLocked
      || !input.policy.requireRejectionReason || !input.policy.requireDeferReason
      || maximumDeferDays < 1 || maximumDeferDays > 180 || openRetentionDays < 1 || openRetentionDays > 180
      || terminalRetentionDays < 1 || terminalRetentionDays > 365 || draftDisplayLimitBytes < 1024
      || draftDisplayLimitBytes > 16_384 || diffDisplayLimitBytes < 4096 || diffDisplayLimitBytes > 65_536) {
      return failure("invalid_input", "invalid_policy");
    }
    const updated = { ...input.policy, schemaVersion: 1 as const, workspaceId: input.workspaceId, revision: current.revision + 1 };
    setPolicy(updated);
    for (const candidate of ensureWorkspace(input.workspaceId)) {
      candidate.detail.policyWitnessHash = stableHash(`${input.workspaceId}:policy:${updated.revision}`);
      if (candidate.detail.currentPreview) {
        candidate.detail.currentPreview.invalidatedAtMs = Date.now();
        delete candidate.detail.currentPreview;
        candidate.detail.staleness = [...new Set([...candidate.detail.staleness, "policy_changed" as const])];
      }
    }
    return success(updated);
  },
  async saveSkillCuratorDraft(input) {
    const found = findCandidate(input.candidateId);
    const cached = found && cachedReceipt(found, `draft:${input.idempotencyKey}`);
    if (cached) return cached;
    const candidate = requireCandidate(input);
    if (!isCandidate(candidate)) return candidate;
    if (!["awaiting_draft", "ready_for_review", "apply_failed"].includes(candidate.detail.state)) {
      return failure("not_approvable", "candidate_state_not_approvable", candidate);
    }
    const validation = validateWebDraft(candidate, input);
    if (validation) return validation;
    const revision = (candidate.detail.drafts[0]?.revision ?? 0) + 1;
    const draft: CuratorDraftRevision = {
      draftId: `web-draft-${candidate.detail.candidateId}`,
      revision,
      kind: input.mutation.kind === "learned_guidance" ? "learn_block" : "exact_patch",
      mutation: structuredClone(input.mutation),
      rationale: input.rationale,
      expectedEffectiveChange: input.expectedEffectiveChange,
      bodyHash: stableHash(`${webDraftText(input)}:${revision}`),
      createdAtMs: Date.now(),
    };
    candidate.detail.drafts.unshift(draft);
    candidate.detail.draftReady = true;
    candidate.detail.staleness = candidate.detail.staleness.filter((reason) => reason !== "draft_changed");
    transition(candidate, "ready_for_review", "draft_assessment_completed");
    publishWebCuratorNotification(candidate, "pending_review");
    const value = receipt(candidate, input.idempotencyKey);
    candidate.actionReceipts.set(`draft:${input.idempotencyKey}`, value);
    return success(value);
  },

  async previewSkillCuratorCandidate(input) {
    const found = findCandidate(input.candidateId);
    const existing = found?.previews.get(input.idempotencyKey);
    if (existing) return success(existing);
    const candidate = requireCandidate(input);
    if (!isCandidate(candidate)) return candidate;
    if (candidate.scenario === "supersede_on_preview") {
      transition(candidate, "superseded", "candidate_superseded", "assessment_changed", "system");
      candidate.detail.staleness = ["assessment_changed"];
      publishWebCuratorNotification(candidate, "supersession");
      return failure("stale_conflict", "assessment_superseded", candidate, "assessment_changed");
    }
    if (candidate.pinned) return failure("pinned", "target_pinned", candidate, "target_pinned");
    const draft = candidate.detail.drafts[0];
    if (!draft || !candidate.detail.draftReady || candidate.detail.state !== "ready_for_review") {
      return failure("not_approvable", "draft_not_ready", candidate);
    }
    if (draft.revision !== input.expectedDraftRevision || candidate.detail.assessmentAttemptId !== input.expectedAssessmentId) {
      return failure("stale_conflict", "preview_witness_conflict", candidate, "draft_or_assessment");
    }
    const preview = createPreview(candidate, input);
    candidate.detail.currentPreview = preview;
    candidate.previews.set(input.idempotencyKey, preview);
    appendAudit(candidate, "preview_created");
    return success(preview);
  },

  async approveSkillCuratorCandidate(input) {
    const found = findCandidate(input.candidateId);
    const existing = found?.applications.get(input.idempotencyKey);
    if (existing) return success(existing);
    const candidate = requireCandidate(input);
    if (!isCandidate(candidate)) return candidate;
    const preview = candidate.detail.currentPreview;
    if (!preview) return failure("not_approvable", "current_preview_required", candidate);
    if (Date.now() >= preview.expiresAtMs) {
      candidate.detail.staleness = [...new Set([...candidate.detail.staleness, "preview_expired" as const])];
      delete candidate.detail.currentPreview;
      return failure("preview_expired", "preview_expired", candidate);
    }
    if (input.confirmedPreviewHash !== preview.witnessHash || input.confirmedEffectiveDiffHash !== preview.effectiveDiffHash) {
      return failure("stale_conflict", "preview_witness_conflict", candidate, "preview_witness");
    }
    const failed = candidate.scenario === "application_failure";
    const recovered = candidate.scenario === "recovery";
    transition(candidate, "applying", "candidate_approved");
    transition(candidate, failed ? "apply_failed" : "applied", failed ? "application_failed" : "application_applied");
    const application: CuratorApplicationResult = {
      ...safeState(candidate),
      applicationId: `web-application-${stableHash(input.idempotencyKey)}`,
      status: failed ? "failed" : recovered ? "reconciled" : "applied",
      ...(failed ? { failureCode: "web_mock_overlay_failure" } : {
        overlayRevision: `web-mock-overlay-${candidate.detail.revision}`,
        overlayHistoryId: `web-mock-history-${candidate.detail.candidateId}`,
      }),
    };
    candidate.detail.application = application;
    candidate.applications.set(input.idempotencyKey, application);
    publishWebCuratorNotification(candidate, failed ? "apply_failure" : "apply_success");
    return success(application);
  },

  async rejectSkillCuratorCandidate(input) {
    const found = findCandidate(input.candidateId);
    const cached = found && cachedReceipt(found, `reject:${input.idempotencyKey}`);
    if (cached) return cached;
    const candidate = requireCandidate(input);
    if (!isCandidate(candidate)) return candidate;
    if (["rejected", "applied", "superseded"].includes(candidate.detail.state)) return failure("not_approvable", "terminal_candidate", candidate);
    if ((input.note?.length ?? 0) > 1000) return failure("invalid_input", "decision_note_too_long", candidate);
    transition(candidate, "rejected", "candidate_rejected", input.reason);
    publishWebCuratorNotification(candidate, "rejection");
    const value = receipt(candidate, input.idempotencyKey);
    candidate.actionReceipts.set(`reject:${input.idempotencyKey}`, value);
    return success(value);
  },

  async deferSkillCuratorCandidate(input) {
    const found = findCandidate(input.candidateId);
    const cached = found && cachedReceipt(found, `defer:${input.idempotencyKey}`);
    if (cached) return cached;
    const candidate = requireCandidate(input);
    if (!isCandidate(candidate)) return candidate;
    if (!["awaiting_draft", "ready_for_review"].includes(candidate.detail.state)) return failure("not_approvable", "candidate_cannot_defer", candidate);
    const maximum = Date.now() + getPolicy(candidate.detail.workspaceId).maximumDeferDays * 86_400_000;
    if ((input.note?.length ?? 0) > 1000 || (input.reviewAfterMs !== undefined && (input.reviewAfterMs < Date.now() + 86_400_000 || input.reviewAfterMs > maximum))) {
      return failure("invalid_input", "invalid_deferral", candidate);
    }
    transition(candidate, "deferred", "candidate_deferred", input.reason);
    publishWebCuratorNotification(candidate, "deferral_date");
    const value = receipt(candidate, input.idempotencyKey);
    candidate.actionReceipts.set(`defer:${input.idempotencyKey}`, value);
    return success(value);
  },

  async resumeSkillCuratorCandidate(input) {
    const found = findCandidate(input.candidateId);
    const cached = found && cachedReceipt(found, `resume:${input.idempotencyKey}`);
    if (cached) return cached;
    const candidate = requireCandidate(input);
    if (!isCandidate(candidate)) return candidate;
    if (candidate.detail.state !== "deferred") return failure("not_approvable", "candidate_not_deferred", candidate);
    if (input.expectedCandidateHash !== candidate.detail.witnessHash || input.expectedPolicyHash !== candidate.detail.policyWitnessHash) {
      return failure("stale_conflict", "resume_witness_conflict", candidate, "resume_witness");
    }
    const draft = candidate.detail.drafts[0];
    if ((input.expectedDraftRevision !== undefined && input.expectedDraftRevision !== draft?.revision)
      || (input.expectedAssessmentId !== undefined && input.expectedAssessmentId !== candidate.detail.assessmentAttemptId)) {
      return failure("stale_conflict", "resume_draft_conflict", candidate, "draft_or_assessment");
    }
    transition(candidate, candidate.detail.draftReady ? "ready_for_review" : "awaiting_draft", "candidate_resumed");
    const value = receipt(candidate, input.idempotencyKey);
    candidate.actionReceipts.set(`resume:${input.idempotencyKey}`, value);
    return success(value);
  },

  async retrySkillCuratorApplication(input) {
    const found = findCandidate(input.candidateId);
    const cached = found && cachedReceipt(found, `retry:${input.idempotencyKey}`);
    if (cached) return cached;
    const candidate = requireCandidate(input);
    if (!isCandidate(candidate)) return candidate;
    if (candidate.detail.state !== "apply_failed") return failure("not_approvable", "application_not_failed", candidate);
    transition(candidate, "ready_for_review", "application_retry_prepared");
    delete candidate.detail.application;
    const value = receipt(candidate, input.idempotencyKey);
    candidate.actionReceipts.set(`retry:${input.idempotencyKey}`, value);
    return success(value);
  },
};
