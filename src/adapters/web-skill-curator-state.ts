import type {
  CuratorActionReceipt,
  CuratorApplicationResult,
  CuratorAuditEvent,
  CuratorCandidateDetail,
  CuratorCandidateState,
  CuratorError,
  CuratorPolicy,
  CuratorPreview,
  CuratorResult,
  CuratorSafeState,
} from "../types/skill-curator";

export interface WebCuratorCandidate {
  detail: CuratorCandidateDetail;
  scenario: string;
  pinned: boolean;
  notificationPending: boolean;
  audit: CuratorAuditEvent[];
  actionReceipts: Map<string, CuratorActionReceipt>;
  previews: Map<string, CuratorPreview>;
  applications: Map<string, CuratorApplicationResult>;
}

const candidates = new Map<string, WebCuratorCandidate>();
const workspaceCandidates = new Map<string, string[]>();
const policies = new Map<string, CuratorPolicy>();

export function stableHash(value: string): string {
  let hash = 2_166_136_261;
  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16_777_619);
  }
  return `web-${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

function scenarioFrom(workspaceId: string): string {
  if (workspaceId.startsWith("mock://")) return workspaceId.slice(7).replaceAll("-", "_");
  const stored = typeof localStorage === "undefined" ? null : localStorage.getItem("vanehub.skillCuratorScenario");
  return stored?.replaceAll("-", "_") ?? "deterministic";
}

export function defaultPolicy(workspaceId: string): CuratorPolicy {
  return {
    schemaVersion: 1,
    workspaceId,
    enqueueRoutes: ["advance", "needs_human_review"],
    requireRejectionReason: true,
    requireDeferReason: true,
    maximumDeferDays: 180,
    openRetentionDays: 180,
    terminalRetentionDays: 365,
    notificationsEnabled: true,
    digestEnabled: false,
    draftDisplayLimitBytes: 16_384,
    diffDisplayLimitBytes: 65_536,
    revision: 1,
  };
}

function fixture(workspaceId: string, scenario: string, ordinal = 0): WebCuratorCandidate {
  const candidateId = `web-curator-${stableHash(`${workspaceId}:${scenario}:${ordinal}`)}`;
  const createdAtMs = Date.now() - (ordinal + 1) * 60_000;
  const state: CuratorCandidateState = scenario === "superseded" ? "superseded" : "awaiting_draft";
  const policy = getPolicy(workspaceId);
  const detail: CuratorCandidateDetail = {
    candidateId,
    targetSkillId: ordinal === 0 ? "review" : `review-${ordinal + 1}`,
    state,
    route: ordinal % 2 === 0 ? "advance" : "needs_human_review",
    risk: scenario === "high_risk" ? "high" : ordinal % 2 === 0 ? "low" : "medium",
    draftReady: false,
    staleness: scenario === "superseded" ? ["assessment_changed"] : [],
    revision: 1,
    updatedAtMs: createdAtMs,
    workspaceId,
    seedId: `web-seed-${ordinal + 1}`,
    assessmentAttemptId: `web-assessment-${scenario}-${ordinal + 1}`,
    assessmentRevision: "assessment-revision-1",
    targetRevision: "target-revision-1",
    overlayScope: "project",
    confidence: scenario === "high_risk" ? "medium" : "high",
    evidenceSources: [{ evidenceId: `web-evidence-${ordinal + 1}`, evidenceRevision: "1", lineageHash: stableHash(`lineage-${ordinal}`) }],
    qualityChecks: Array.from({ length: 9 }, (_, index) => ({
      code: `quality-check-${index + 1}`,
      result: scenario === "high_risk" && index === 7 ? "review" as const : "pass" as const,
      reasonCode: scenario === "high_risk" && index === 7 ? "manual_review_required" : "check_passed",
    })),
    witnessHash: stableHash(`${candidateId}:1`),
    policyWitnessHash: stableHash(`${workspaceId}:policy:${policy.revision}`),
    drafts: [],
    createdAtMs,
  };
  return {
    detail,
    scenario,
    pinned: scenario === "pinned",
    notificationPending: false,
    audit: [{
      sequence: 1,
      eventKind: scenario === "superseded" ? "candidate_superseded" : "candidate_intake",
      actorClass: "system",
      occurredAtMs: createdAtMs,
      nextState: state,
      objectRevision: 1,
      ...(scenario === "superseded" ? { reasonCode: "assessment_changed" } : {}),
      eventHash: stableHash(`${candidateId}:event:1`),
    }],
    actionReceipts: new Map(),
    previews: new Map(),
    applications: new Map(),
  };
}

export function ensureWorkspace(workspaceId: string): WebCuratorCandidate[] {
  const existing = workspaceCandidates.get(workspaceId);
  if (existing) return existing.flatMap((id) => candidates.get(id) ?? []);
  const scenario = scenarioFrom(workspaceId);
  const created = scenario === "empty" ? [] : Array.from(
    { length: scenario === "pagination" ? 3 : 1 },
    (_, index) => fixture(workspaceId, scenario, index),
  );
  for (const candidate of created) candidates.set(candidate.detail.candidateId, candidate);
  workspaceCandidates.set(workspaceId, created.map(({ detail }) => detail.candidateId));
  return created;
}

export function findCandidate(candidateId: string): WebCuratorCandidate | undefined {
  return candidates.get(candidateId);
}

export function getPolicy(workspaceId: string): CuratorPolicy {
  const existing = policies.get(workspaceId);
  if (existing) return existing;
  const policy = defaultPolicy(workspaceId);
  policies.set(workspaceId, policy);
  return policy;
}

export function setPolicy(policy: CuratorPolicy): void {
  policies.set(policy.workspaceId, policy);
}

export function safeState(candidate: WebCuratorCandidate): CuratorSafeState {
  const { candidateId, revision, state, witnessHash, policyWitnessHash, currentPreview } = candidate.detail;
  return {
    candidateId,
    revision,
    state,
    witnessHash,
    policyWitnessHash,
    ...(currentPreview === undefined ? {} : { currentPreviewId: currentPreview.previewId }),
  };
}

export function failure<T>(code: CuratorError["code"], message: string, candidate?: WebCuratorCandidate, reasonCode?: string): CuratorResult<T> {
  return {
    ok: false,
    error: {
      code,
      message,
      ...(candidate === undefined ? {} : { current: safeState(candidate) }),
      ...(reasonCode === undefined ? {} : { reasonCode }),
    },
  };
}

export function success<T>(value: T): CuratorResult<T> {
  return { ok: true, value: structuredClone(value) };
}

export function appendAudit(
  candidate: WebCuratorCandidate,
  eventKind: string,
  priorState = candidate.detail.state,
  actorClass: CuratorAuditEvent["actorClass"] = "web_mock_interactive_user",
  reasonCode?: string,
): void {
  const sequence = candidate.audit.length + 1;
  candidate.audit.push({
    sequence,
    eventKind,
    actorClass,
    occurredAtMs: Date.now(),
    priorState,
    nextState: candidate.detail.state,
    objectRevision: candidate.detail.revision,
    ...(reasonCode === undefined ? {} : { reasonCode }),
    eventHash: stableHash(`${candidate.detail.candidateId}:event:${sequence}`),
  });
}

export function transition(
  candidate: WebCuratorCandidate,
  nextState: CuratorCandidateState,
  eventKind: string,
  reasonCode?: string,
  actorClass: CuratorAuditEvent["actorClass"] = "web_mock_interactive_user",
): void {
  const priorState = candidate.detail.state;
  candidate.detail.revision += 1;
  candidate.detail.state = nextState;
  candidate.detail.updatedAtMs = Date.now();
  candidate.detail.witnessHash = stableHash(`${candidate.detail.candidateId}:${candidate.detail.revision}:${nextState}`);
  if (candidate.detail.currentPreview) candidate.detail.currentPreview.invalidatedAtMs = Date.now();
  delete candidate.detail.currentPreview;
  appendAudit(candidate, eventKind, priorState, actorClass, reasonCode);
}

export function resetWebSkillCuratorForTest(): void {
  candidates.clear();
  workspaceCandidates.clear();
  policies.clear();
}
