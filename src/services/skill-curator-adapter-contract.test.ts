import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SkillCuratorService } from "./skill-curator-service";
import type {
  CuratorCandidateDetail,
  CuratorError,
  CuratorResult,
  PreviewCuratorCandidateInput,
  SaveCuratorDraftInput,
} from "../types/skill-curator";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriSkillCuratorClient } from "../adapters/tauri-skill-curator-client";
import {
  resetWebSkillCuratorForTest,
  webSkillCuratorClient,
} from "../adapters/web-skill-curator-client";

const now = 1_780_000_000_000;

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null) throw new Error("native arguments must be an object");
  return value as Record<string, unknown>;
}

function input<T>(args: unknown): T {
  return record(args).input as T;
}

async function nativeValue<T>(result: Promise<CuratorResult<T>>): Promise<T> {
  const settled = await result;
  if (settled.ok) return settled.value;
  throw JSON.stringify(settled.error);
}

async function nativeBridge(command: string, args?: unknown): Promise<unknown> {
  switch (command) {
    case "query_skill_curator_queue":
      return nativeValue(webSkillCuratorClient.querySkillCuratorQueue(input(args)));
    case "get_skill_curator_candidate":
      return nativeValue(webSkillCuratorClient.getSkillCuratorCandidate(record(args).candidateId as string));
    case "query_skill_curator_audit": {
      const query = input<{ candidateId: string; cursor?: string }>(args);
      return nativeValue(webSkillCuratorClient.querySkillCuratorAudit(query.candidateId, query.cursor));
    }
    case "get_skill_curator_policy":
      return nativeValue(webSkillCuratorClient.getSkillCuratorPolicy(record(args).workspaceId as string));
    case "update_skill_curator_policy":
      return nativeValue(webSkillCuratorClient.updateSkillCuratorPolicy(input(args)));
    case "save_skill_curator_draft":
      return nativeValue(webSkillCuratorClient.saveSkillCuratorDraft(input(args)));
    case "preview_skill_curator_candidate":
      return nativeValue(webSkillCuratorClient.previewSkillCuratorCandidate(input(args)));
    case "approve_skill_curator_candidate":
      return nativeValue(webSkillCuratorClient.approveSkillCuratorCandidate(input(args)));
    case "reject_skill_curator_candidate":
      return nativeValue(webSkillCuratorClient.rejectSkillCuratorCandidate(input(args)));
    case "defer_skill_curator_candidate":
      return nativeValue(webSkillCuratorClient.deferSkillCuratorCandidate(input(args)));
    case "resume_skill_curator_candidate":
      return nativeValue(webSkillCuratorClient.resumeSkillCuratorCandidate(input(args)));
    case "retry_skill_curator_application":
      return nativeValue(webSkillCuratorClient.retrySkillCuratorApplication(input(args)));
    default:
      throw new Error(`unexpected native command: ${command}`);
  }
}

async function value<T>(result: Promise<CuratorResult<T>>): Promise<T> {
  const settled = await result;
  if (!settled.ok) throw new Error(`expected success, received ${settled.error.code}`);
  return settled.value;
}

function error(result: CuratorResult<unknown>): CuratorError {
  if (result.ok) throw new Error("expected failure");
  return result.error;
}

async function candidate(client: SkillCuratorService, scenario: string): Promise<CuratorCandidateDetail> {
  const page = await value(client.querySkillCuratorQueue({ workspaceId: `mock://${scenario}` }));
  return value(client.getSkillCuratorCandidate(page.items[0].candidateId));
}

function draft(detail: CuratorCandidateDetail, idempotencyKey: string, guidance = "Prefer bounded changes."): SaveCuratorDraftInput {
  return {
    schemaVersion: 1,
    candidateId: detail.candidateId,
    expectedCandidateRevision: detail.revision,
    idempotencyKey,
    mutation: { kind: "learned_guidance", guidance },
    rationale: "Deterministic evidence supports this guidance.",
    expectedEffectiveChange: "Adds one non-executable guidance block.",
  };
}

async function ready(client: SkillCuratorService, scenario: string, guidance?: string): Promise<CuratorCandidateDetail> {
  const detail = await candidate(client, scenario);
  await value(client.saveSkillCuratorDraft(draft(detail, "draft-1", guidance)));
  return value(client.getSkillCuratorCandidate(detail.candidateId));
}

function previewInput(detail: CuratorCandidateDetail): PreviewCuratorCandidateInput {
  return {
    candidateId: detail.candidateId,
    expectedCandidateRevision: detail.revision,
    expectedDraftRevision: detail.drafts[0].revision,
    expectedAssessmentId: detail.assessmentAttemptId,
    idempotencyKey: "preview-1",
  };
}

const adapters = [
  ["Web", webSkillCuratorClient],
  ["desktop", tauriSkillCuratorClient],
] as const;

afterEach(() => vi.useRealTimers());

describe.each(adapters)("Skill Curator %s adapter contract", (_name, client) => {
  beforeEach(() => {
    resetWebSkillCuratorForTest();
    invokeMock.mockReset();
    invokeMock.mockImplementation(nativeBridge);
    vi.useFakeTimers();
    vi.setSystemTime(now);
  });

  it("paginates queue and audit records with explicit completeness", async () => {
    const first = await value(client.querySkillCuratorQueue({ workspaceId: "mock://pagination", limit: 2 }));
    const second = await value(client.querySkillCuratorQueue({
      workspaceId: "mock://pagination",
      limit: 2,
      cursor: first.nextCursor,
    }));
    expect(first).toMatchObject({ totalCount: 3, nextCursor: "2", complete: false });
    expect(second).toMatchObject({ totalCount: 3, complete: true });
    expect([...first.items, ...second.items]).toHaveLength(3);

    let detail = await candidate(client, "audit_pagination");
    for (let index = 0; index < 20; index += 1) {
      const receipt = await value(client.saveSkillCuratorDraft(draft(detail, `audit-draft-${index}`)));
      detail = await value(client.getSkillCuratorCandidate(receipt.candidateId));
    }
    const auditFirst = await value(client.querySkillCuratorAudit(detail.candidateId));
    const auditSecond = await value(client.querySkillCuratorAudit(detail.candidateId, auditFirst.nextCursor));
    expect(auditFirst).toMatchObject({ nextCursor: "20", complete: false });
    expect(auditFirst.items).toHaveLength(20);
    expect(auditSecond).toMatchObject({ complete: true });
    expect(auditSecond.items).toHaveLength(1);
  });

  it("bounds preview diffs and exposes truncation metadata", async () => {
    const workspaceId = "mock://bounded_diff";
    const policy = await value(client.getSkillCuratorPolicy(workspaceId));
    await value(client.updateSkillCuratorPolicy({
      workspaceId,
      expectedRevision: policy.revision,
      policy: {
        enqueueRoutes: policy.enqueueRoutes,
        requireRejectionReason: policy.requireRejectionReason,
        requireDeferReason: policy.requireDeferReason,
        maximumDeferDays: policy.maximumDeferDays,
        openRetentionDays: policy.openRetentionDays,
        terminalRetentionDays: policy.terminalRetentionDays,
        notificationsEnabled: policy.notificationsEnabled,
        digestEnabled: policy.digestEnabled,
        draftDisplayLimitBytes: policy.draftDisplayLimitBytes,
        diffDisplayLimitBytes: 4096,
      },
    }));
    const detail = await ready(client, "bounded_diff", "x".repeat(5000));
    const preview = await value(client.previewSkillCuratorCandidate(previewInput(detail)));

    for (const projection of [preview.diffs.currentToProposed, preview.diffs.baseToProposed]) {
      expect(projection).toMatchObject({ complete: false, nextCursor: "4096", addedCharacters: 5000 });
      expect(projection.hunks[0].after).toMatchObject({ totalCharacters: 5000, truncated: true });
      expect(projection.hunks[0].after.content).toHaveLength(4096);
    }
  });

  it("enforces versioned transitions and terminal rejection", async () => {
    const initial = await candidate(client, "transitions");
    const drafted = await value(client.saveSkillCuratorDraft(draft(initial, "transition-draft")));
    const deferred = await value(client.deferSkillCuratorCandidate({
      candidateId: drafted.candidateId,
      expectedCandidateRevision: drafted.revision,
      idempotencyKey: "transition-defer",
      reason: "need_more_evidence",
    }));
    const resumed = await value(client.resumeSkillCuratorCandidate({
      candidateId: deferred.candidateId,
      expectedCandidateRevision: deferred.revision,
      expectedCandidateHash: deferred.witnessHash,
      expectedPolicyHash: deferred.policyWitnessHash,
      expectedDraftRevision: 1,
      expectedAssessmentId: initial.assessmentAttemptId,
      idempotencyKey: "transition-resume",
    }));
    const rejected = await value(client.rejectSkillCuratorCandidate({
      candidateId: resumed.candidateId,
      expectedCandidateRevision: resumed.revision,
      idempotencyKey: "transition-reject",
      reason: "not_useful",
    }));
    const terminal = await client.deferSkillCuratorCandidate({
      candidateId: rejected.candidateId,
      expectedCandidateRevision: rejected.revision,
      idempotencyKey: "terminal-defer",
      reason: "lower_priority",
    });

    expect([drafted.state, deferred.state, resumed.state, rejected.state]).toEqual([
      "ready_for_review", "deferred", "ready_for_review", "rejected",
    ]);
    expect(error(terminal)).toMatchObject({ code: "not_approvable", current: { state: "rejected" } });
  });

  it("returns current safe state for stale versions", async () => {
    const initial = await candidate(client, "conflict");
    await value(client.saveSkillCuratorDraft(draft(initial, "conflict-draft")));
    const stale = await client.rejectSkillCuratorCandidate({
      candidateId: initial.candidateId,
      expectedCandidateRevision: initial.revision,
      idempotencyKey: "stale-reject",
      reason: "not_useful",
    });

    expect(error(stale)).toMatchObject({
      code: "stale_conflict",
      message: "candidate_revision_conflict",
      reasonCode: "candidate_revision",
      current: { candidateId: initial.candidateId, revision: 2, state: "ready_for_review" },
    });
  });

  it("deduplicates repeated action keys without another transition", async () => {
    const initial = await candidate(client, "idempotency");
    const request = draft(initial, "same-action");
    const first = await value(client.saveSkillCuratorDraft(request));
    const repeated = await value(client.saveSkillCuratorDraft(request));
    const detail = await value(client.getSkillCuratorCandidate(initial.candidateId));

    expect(first.duplicate).toBe(false);
    expect(repeated).toMatchObject({ actionId: first.actionId, revision: first.revision, duplicate: true });
    expect(detail.drafts).toHaveLength(1);
    expect(detail.revision).toBe(first.revision);
  });
});

describe("Skill Curator desktop/Web error parity", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(nativeBridge);
    vi.useFakeTimers();
    vi.setSystemTime(now);
  });

  async function staleError(client: SkillCuratorService): Promise<CuratorError> {
    resetWebSkillCuratorForTest();
    const initial = await candidate(client, "error_parity");
    await value(client.saveSkillCuratorDraft(draft(initial, "parity-draft")));
    return error(await client.rejectSkillCuratorCandidate({
      candidateId: initial.candidateId,
      expectedCandidateRevision: initial.revision,
      idempotencyKey: "parity-reject",
      reason: "not_useful",
    }));
  }

  async function pinnedError(client: SkillCuratorService): Promise<CuratorError> {
    resetWebSkillCuratorForTest();
    const detail = await ready(client, "pinned");
    return error(await client.previewSkillCuratorCandidate(previewInput(detail)));
  }

  it("normalizes validation, conflict, pinned, and not-found failures identically", async () => {
    const webErrors = [
      error(await webSkillCuratorClient.querySkillCuratorQueue({ workspaceId: "", limit: 0 })),
      error(await webSkillCuratorClient.getSkillCuratorCandidate("missing")),
      await staleError(webSkillCuratorClient),
      await pinnedError(webSkillCuratorClient),
    ];
    const desktopErrors = [
      error(await tauriSkillCuratorClient.querySkillCuratorQueue({ workspaceId: "", limit: 0 })),
      error(await tauriSkillCuratorClient.getSkillCuratorCandidate("missing")),
      await staleError(tauriSkillCuratorClient),
      await pinnedError(tauriSkillCuratorClient),
    ];

    expect(desktopErrors).toEqual(webErrors);
    expect(webErrors.map(({ code }) => code)).toEqual([
      "invalid_input", "not_found", "stale_conflict", "pinned",
    ]);
  });
});
