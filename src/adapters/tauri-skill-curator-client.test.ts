import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ApproveCuratorCandidateInput,
  DeferCuratorCandidateInput,
  PreviewCuratorCandidateInput,
  RejectCuratorCandidateInput,
  ResumeCuratorCandidateInput,
  SaveCuratorDraftInput,
  UpdateCuratorPolicyInput,
} from "../types/skill-curator";

const { invokeMock, listenMock } = vi.hoisted(() => ({ invokeMock: vi.fn(), listenMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriSkillCuratorClient } from "./tauri-skill-curator-client";
import { tauriAgentClient } from "../services/tauri-agent-client";

const versioned = { candidateId: "candidate-1", expectedCandidateRevision: 4, idempotencyKey: "action-1" };
const draft: SaveCuratorDraftInput = {
  ...versioned,
  schemaVersion: 1,
  mutation: { kind: "learned_guidance", guidance: "Prefer bounded changes." },
  rationale: "Evidence supports the guidance.",
  expectedEffectiveChange: "Adds one guidance block.",
};
const preview: PreviewCuratorCandidateInput = {
  ...versioned,
  expectedDraftRevision: 2,
  expectedAssessmentId: "assessment-1",
};
const approve: ApproveCuratorCandidateInput = {
  ...versioned,
  confirmedPreviewHash: "preview-hash",
  confirmedEffectiveDiffHash: "diff-hash",
};
const reject: RejectCuratorCandidateInput = { ...versioned, reason: "not_useful" };
const defer: DeferCuratorCandidateInput = { ...versioned, reason: "need_more_evidence" };
const resume: ResumeCuratorCandidateInput = {
  ...versioned,
  expectedCandidateHash: "candidate-hash",
  expectedPolicyHash: "policy-hash",
};
const updatePolicy: UpdateCuratorPolicyInput = {
  workspaceId: "workspace-1",
  expectedRevision: 3,
  policy: {
    enqueueRoutes: ["advance", "needs_human_review"],
    requireRejectionReason: true,
    requireDeferReason: true,
    maximumDeferDays: 180,
    openRetentionDays: 180,
    terminalRetentionDays: 365,
    notificationsEnabled: true,
    digestEnabled: false,
    draftDisplayLimitBytes: 8192,
    diffDisplayLimitBytes: 16384,
  },
};

const operations = [
  ["query_skill_curator_queue", { input: { workspaceId: "workspace-1" } }, () => tauriSkillCuratorClient.querySkillCuratorQueue({ workspaceId: "workspace-1" })],
  ["get_skill_curator_candidate", { candidateId: "candidate-1" }, () => tauriSkillCuratorClient.getSkillCuratorCandidate("candidate-1")],
  ["query_skill_curator_audit", { input: { candidateId: "candidate-1", cursor: "10" } }, () => tauriSkillCuratorClient.querySkillCuratorAudit("candidate-1", "10")],
  ["get_skill_curator_policy", { workspaceId: "workspace-1" }, () => tauriSkillCuratorClient.getSkillCuratorPolicy("workspace-1")],
  ["update_skill_curator_policy", { input: updatePolicy }, () => tauriSkillCuratorClient.updateSkillCuratorPolicy(updatePolicy)],
  ["save_skill_curator_draft", { input: draft }, () => tauriSkillCuratorClient.saveSkillCuratorDraft(draft)],
  ["preview_skill_curator_candidate", { input: preview }, () => tauriSkillCuratorClient.previewSkillCuratorCandidate(preview)],
  ["approve_skill_curator_candidate", { input: approve }, () => tauriSkillCuratorClient.approveSkillCuratorCandidate(approve)],
  ["reject_skill_curator_candidate", { input: reject }, () => tauriSkillCuratorClient.rejectSkillCuratorCandidate(reject)],
  ["defer_skill_curator_candidate", { input: defer }, () => tauriSkillCuratorClient.deferSkillCuratorCandidate(defer)],
  ["resume_skill_curator_candidate", { input: resume }, () => tauriSkillCuratorClient.resumeSkillCuratorCandidate(resume)],
  ["retry_skill_curator_application", { input: versioned }, () => tauriSkillCuratorClient.retrySkillCuratorApplication(versioned)],
] as const;

describe("Tauri Skill Curator client", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  it("maps every operation and wraps successful native payloads", async () => {
    invokeMock.mockResolvedValue({ native: true });

    const results = [];
    for (const operation of operations) results.push(await operation[2]());

    expect(invokeMock.mock.calls).toEqual(operations.map(([command, args]) => [command, args]));
    expect(results).toEqual(operations.map(() => ({ ok: true, value: { native: true } })));
  });

  it("preserves typed native conflicts and their safe current state", async () => {
    invokeMock.mockRejectedValueOnce({
      code: "stale_conflict",
      message: "candidate_revision_conflict",
      current: {
        candidateId: "candidate-1",
        revision: 5,
        state: "ready_for_review",
        witnessHash: "candidate-hash-5",
        policyWitnessHash: "policy-hash-3",
      },
      reasonCode: "candidate_revision",
    });

    await expect(tauriSkillCuratorClient.rejectSkillCuratorCandidate(reject)).resolves.toEqual({
      ok: false,
      error: {
        code: "stale_conflict",
        message: "candidate_revision_conflict",
        current: {
          candidateId: "candidate-1",
          revision: 5,
          state: "ready_for_review",
          witnessHash: "candidate-hash-5",
          policyWitnessHash: "policy-hash-3",
        },
        reasonCode: "candidate_revision",
      },
    });
  });

  it("maps malformed native failures to a stable fail-closed result", async () => {
    invokeMock.mockRejectedValueOnce(new Error("contains unsafe native detail"));

    await expect(tauriSkillCuratorClient.getSkillCuratorCandidate("candidate-1")).resolves.toEqual({
      ok: false,
      error: { code: "storage_unavailable", message: "skill_curator_native_failure" },
    });
  });

  it("isolates Curator database failure from Agent, evidence, assessment, and Overlay consumers", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "query_skill_curator_queue") throw new Error("curator database unavailable");
      if (command === "list_agents") return [{ id: "codex", displayName: "Codex" }];
      if (command === "query_skill_evolution_evidence") return { signalCount: 0 };
      if (command === "get_skill_evolution_assessment") return { attemptId: "assessment-1" };
      if (command === "get_skill_overlay_detail") return { summary: { skillId: "review" } };
      throw new Error(`unexpected command ${command}`);
    });

    await expect(tauriSkillCuratorClient.querySkillCuratorQueue({ workspaceId: "workspace-1" }))
      .resolves.toEqual({ ok: false, error: { code: "storage_unavailable", message: "skill_curator_native_failure" } });
    await expect(tauriAgentClient.listAgents()).resolves.toEqual([{ id: "codex", displayName: "Codex" }]);
    await expect(tauriAgentClient.querySkillEvolutionEvidence({ workspace: "workspace-1" }))
      .resolves.toEqual({ signalCount: 0 });
    await expect(tauriAgentClient.getSkillEvolutionAssessment("assessment-1"))
      .resolves.toEqual({ attemptId: "assessment-1" });
    await expect(tauriAgentClient.getSkillOverlayDetail({ skillId: "review", scope: "project", workspacePath: "workspace-1" }))
      .resolves.toEqual({ summary: { skillId: "review" } });
  });
});
