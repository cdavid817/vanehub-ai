import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CuratorCandidateDetail,
  CuratorPreview,
  CuratorResult,
  SaveCuratorDraftInput,
} from "../types/skill-curator";
import { resetWebSkillCuratorForTest, webAgentClient } from "../services/web-agent-client";

const now = 1_780_000_000_000;

async function value<T>(promise: Promise<CuratorResult<T>>): Promise<T> {
  const result = await promise;
  if (!result.ok) throw new Error(`Expected success, received ${result.error.code}`);
  return result.value;
}

async function candidate(scenario: string): Promise<CuratorCandidateDetail> {
  const page = await value(webAgentClient.querySkillCuratorQueue({ workspaceId: `mock://${scenario}` }));
  return value(webAgentClient.getSkillCuratorCandidate(page.items[0].candidateId));
}

function draftInput(detail: CuratorCandidateDetail, idempotencyKey = "draft-1"): SaveCuratorDraftInput {
  return {
    schemaVersion: 1,
    candidateId: detail.candidateId,
    expectedCandidateRevision: detail.revision,
    idempotencyKey,
    mutation: { kind: "learned_guidance", guidance: "Prefer bounded, evidence-backed changes." },
    rationale: "The deterministic checks support this guidance.",
    expectedEffectiveChange: "Adds one non-executable guidance block.",
  };
}

async function ready(scenario: string): Promise<CuratorCandidateDetail> {
  const initial = await candidate(scenario);
  await value(webAgentClient.saveSkillCuratorDraft(draftInput(initial)));
  return value(webAgentClient.getSkillCuratorCandidate(initial.candidateId));
}

async function preview(detail: CuratorCandidateDetail, idempotencyKey = "preview-1"): Promise<CuratorPreview> {
  return value(webAgentClient.previewSkillCuratorCandidate({
    candidateId: detail.candidateId,
    expectedCandidateRevision: detail.revision,
    expectedDraftRevision: detail.drafts[0].revision,
    expectedAssessmentId: detail.assessmentAttemptId,
    idempotencyKey,
  }));
}

describe("Web Skill Curator client", () => {
  beforeEach(() => {
    resetWebSkillCuratorForTest();
    vi.useFakeTimers();
    vi.setSystemTime(now);
  });

  afterEach(() => vi.useRealTimers());

  it("runs draft, defer, resume, preview and mock application as versioned transitions", async () => {
    const initial = await candidate("deterministic");
    await value(webAgentClient.saveSkillCuratorDraft(draftInput(initial)));
    const stale = await webAgentClient.rejectSkillCuratorCandidate({
      candidateId: initial.candidateId,
      expectedCandidateRevision: initial.revision,
      idempotencyKey: "stale-reject",
      reason: "not_useful",
    });
    expect(stale).toMatchObject({ ok: false, error: { code: "stale_conflict", current: { revision: 2 } } });

    const drafted = await value(webAgentClient.getSkillCuratorCandidate(initial.candidateId));
    const deferred = await value(webAgentClient.deferSkillCuratorCandidate({
      candidateId: drafted.candidateId,
      expectedCandidateRevision: drafted.revision,
      idempotencyKey: "defer-1",
      reason: "need_more_evidence",
    }));
    const resumed = await value(webAgentClient.resumeSkillCuratorCandidate({
      candidateId: deferred.candidateId,
      expectedCandidateRevision: deferred.revision,
      expectedCandidateHash: deferred.witnessHash,
      expectedPolicyHash: deferred.policyWitnessHash,
      expectedDraftRevision: drafted.drafts[0].revision,
      expectedAssessmentId: drafted.assessmentAttemptId,
      idempotencyKey: "resume-1",
    }));
    const resumedDetail = await value(webAgentClient.getSkillCuratorCandidate(resumed.candidateId));
    const currentPreview = await preview(resumedDetail);
    const application = await value(webAgentClient.approveSkillCuratorCandidate({
      candidateId: resumed.candidateId,
      expectedCandidateRevision: resumed.revision,
      confirmedPreviewHash: currentPreview.witnessHash,
      confirmedEffectiveDiffHash: currentPreview.effectiveDiffHash,
      idempotencyKey: "approve-1",
    }));

    expect(application).toMatchObject({ state: "applied", status: "applied", overlayRevision: "web-mock-overlay-6" });
    expect(application.overlayHistoryId).toContain("web-mock-history-");
    expect((await value(webAgentClient.querySkillCuratorAudit(initial.candidateId))).items.map(({ eventKind }) => eventKind))
      .toEqual([
        "candidate_intake", "draft_assessment_completed", "candidate_deferred", "candidate_resumed",
        "preview_created", "candidate_approved", "application_applied",
      ]);
  });

  it("makes repeated action keys idempotent without creating another draft revision", async () => {
    const initial = await candidate("deterministic");
    const input = draftInput(initial);
    const first = await value(webAgentClient.saveSkillCuratorDraft(input));
    const repeated = await value(webAgentClient.saveSkillCuratorDraft(input));
    const detail = await value(webAgentClient.getSkillCuratorCandidate(initial.candidateId));

    expect(first.duplicate).toBe(false);
    expect(repeated).toMatchObject({ actionId: first.actionId, duplicate: true, revision: first.revision });
    expect(detail.drafts).toHaveLength(1);
  });

  it("refuses pinned preview and supersedes a candidate when assessment drift is simulated", async () => {
    const pinned = await ready("pinned");
    await expect(preview(pinned)).rejects.toThrow("Expected success, received pinned");

    const drifting = await ready("supersede_on_preview");
    const result = await webAgentClient.previewSkillCuratorCandidate({
      candidateId: drifting.candidateId,
      expectedCandidateRevision: drifting.revision,
      expectedDraftRevision: drifting.drafts[0].revision,
      expectedAssessmentId: drifting.assessmentAttemptId,
      idempotencyKey: "drift-preview",
    });
    expect(result).toMatchObject({ ok: false, error: { code: "stale_conflict", current: { state: "superseded" } } });
  });

  it("expires previews after fifteen minutes", async () => {
    const detail = await ready("deterministic");
    const currentPreview = await preview(detail);
    vi.advanceTimersByTime(15 * 60_000);

    const result = await webAgentClient.approveSkillCuratorCandidate({
      candidateId: detail.candidateId,
      expectedCandidateRevision: detail.revision,
      confirmedPreviewHash: currentPreview.witnessHash,
      confirmedEffectiveDiffHash: currentPreview.effectiveDiffHash,
      idempotencyKey: "expired-approval",
    });
    expect(result).toMatchObject({ ok: false, error: { code: "preview_expired" } });
  });

  it("requires a fresh preview after application failure and supports mock recovery provenance", async () => {
    const failedDetail = await ready("application_failure");
    const failedPreview = await preview(failedDetail);
    const failed = await value(webAgentClient.approveSkillCuratorCandidate({
      candidateId: failedDetail.candidateId,
      expectedCandidateRevision: failedDetail.revision,
      confirmedPreviewHash: failedPreview.witnessHash,
      confirmedEffectiveDiffHash: failedPreview.effectiveDiffHash,
      idempotencyKey: "failed-approval",
    }));
    const retry = await value(webAgentClient.retrySkillCuratorApplication({
      candidateId: failed.candidateId,
      expectedCandidateRevision: failed.revision,
      idempotencyKey: "retry-1",
    }));
    expect(failed).toMatchObject({ state: "apply_failed", status: "failed", failureCode: "web_mock_overlay_failure" });
    expect(retry).toMatchObject({ state: "ready_for_review" });
    expect((await value(webAgentClient.getSkillCuratorCandidate(retry.candidateId))).currentPreview).toBeUndefined();

    const recoveryDetail = await ready("recovery");
    const recoveryPreview = await preview(recoveryDetail);
    const recovered = await value(webAgentClient.approveSkillCuratorCandidate({
      candidateId: recoveryDetail.candidateId,
      expectedCandidateRevision: recoveryDetail.revision,
      confirmedPreviewHash: recoveryPreview.witnessHash,
      confirmedEffectiveDiffHash: recoveryPreview.effectiveDiffHash,
      idempotencyKey: "recovery-approval",
    }));
    expect(recovered).toMatchObject({ state: "applied", status: "reconciled" });
    expect(recovered.overlayRevision).toMatch(/^web-mock-overlay-/);
  });

  it("filters and paginates queue fixtures with explicit completeness", async () => {
    const first = await value(webAgentClient.querySkillCuratorQueue({ workspaceId: "mock://pagination", limit: 2 }));
    const second = await value(webAgentClient.querySkillCuratorQueue({
      workspaceId: "mock://pagination",
      limit: 2,
      cursor: first.nextCursor,
    }));
    expect(first).toMatchObject({ totalCount: 3, complete: false, nextCursor: "2" });
    expect(first.items).toHaveLength(2);
    expect(second).toMatchObject({ totalCount: 3, complete: true });
    expect(second.items).toHaveLength(1);
  });
});
