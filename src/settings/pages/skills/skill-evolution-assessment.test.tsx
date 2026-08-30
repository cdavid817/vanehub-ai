// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import "../../../i18n";
import type { AssessmentDetail } from "../../../services/skill-assessment-service";
import type { Skill } from "../../../types/skill";

const service = vi.hoisted(() => ({
  getSkillEvolutionAssessment: vi.fn(),
  getSkillEvolutionAssessmentPolicy: vi.fn(),
  querySkillEvolutionAssessments: vi.fn(),
  scheduleSkillEvolutionReassessment: vi.fn(),
  updateSkillEvolutionAssessmentConsent: vi.fn(),
}));
vi.mock("../../../services/runtime-agent-client", () => ({ agentService: service }));

import { SkillEvolutionAssessment } from "./skill-evolution-assessment";

const summary = {
  attemptId: "attempt-1", seedId: "seed-1", seedRevision: "seed-r1", status: "completed" as const,
  classification: "selected" as const, route: "advance" as const, confidence: "high" as const,
  risk: "low" as const, isCurrent: true, winningRule: "quality_gate_lattice", createdAtMs: 1_776_000_000_000,
};
const checkKinds = ["privacy_residue", "evidence_sufficiency", "duplicate_knowledge", "transient_incident", "guidance_specificity", "evidence_consistency", "target_compatibility", "executable_content_risk", "target_lifecycle_mutability"];
const detail: AssessmentDetail = {
  ...summary,
  targets: [{ ordinal: 0, skillId: "review", skillType: "role", revisionHash: "revision-hash-1", scope: "project", lifecycle: "active", trust: "trusted", score: 91, attribution: "verified", attributionUncertain: false, matchedFeatureClasses: ["capability", "description"], exclusions: [], components: [{ component: "attribution", score: 35 }, { component: "compatibility", score: 20 }] }],
  checks: checkKinds.map((kind, ordinal) => ({ ordinal, kind, result: "pass", severity: "low", reasonCode: "check_passed", evidenceIds: ["signal-1"], routeConstraints: [] })),
  provenance: { deterministic: true, modelEvaluationAllowed: false, modelConsulted: false },
  routeConstraints: [],
  selectionThreshold: { leadingScore: 91, runnerUpScore: 52, margin: 39, selectedMinimum: 60, ambiguousMinimum: 45, requiredMargin: 15 },
  versionWitnesses: { witnessHash: "witness-1", lineageHash: "lineage-1", targetUniverseHash: "universe-1", sanitizerVersion: "sanitizer-v1", selectorPolicyVersion: "selector-v1", gatePolicyVersion: "gates-v1", routingPolicyVersion: "routing-v1", confidencePolicyVersion: "confidence-v1", consentVersion: "disabled" },
};
const skill = { id: "review", workspacePath: null, metadata: { type: "role" } } as Skill;

function renderAssessment() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return render(<QueryClientProvider client={client}><SkillEvolutionAssessment skill={skill} /></QueryClientProvider>);
}

beforeEach(() => {
  vi.clearAllMocks();
  service.querySkillEvolutionAssessments.mockResolvedValue({ items: [summary] });
  service.getSkillEvolutionAssessment.mockResolvedValue(detail);
  service.getSkillEvolutionAssessmentPolicy.mockResolvedValue({ evaluatorPolicyVersion: "structured-evaluator-v1", disclosureVersion: "assessment-disclosure-v1", modelEvaluationEnabled: false, providerAvailable: true, changedAtMs: 0 });
  service.updateSkillEvolutionAssessmentConsent.mockResolvedValue({ evaluatorPolicyVersion: "structured-evaluator-v1", disclosureVersion: "assessment-disclosure-v1", modelEvaluationEnabled: true, providerAvailable: true, changedAtMs: 1 });
  service.scheduleSkillEvolutionReassessment.mockResolvedValue({ queueId: "queue-1", status: "scheduled" });
});

describe("SkillEvolutionAssessment", () => {
  it("renders the complete safe explanation without mutation controls", async () => {
    const user = userEvent.setup();
    renderAssessment();
    expect(await screen.findByRole("heading", { name: "Skill 评估" })).toBeTruthy();
    expect(await screen.findByText("可进入后续治理")).toBeTruthy();
    expect(screen.getByText("9 项中 9 项通过")).toBeTruthy();
    expect(screen.getByText("39/15")).toBeTruthy();
    expect(screen.getByText("九项确定性质量检查")).toBeTruthy();
    expect(screen.getAllByText("通过")).toHaveLength(9);
    await user.click(screen.getByText("review"));
    expect(screen.getByText(/revision-hash-1/)).toBeTruthy();
    expect(screen.getByText(/capability, description/)).toBeTruthy();
    await user.click(screen.getByText("隐私残留"));
    expect(screen.getAllByText("signal-1").length).toBeGreaterThan(0);
    expect(screen.queryByRole("button", { name: /批准|拒绝|应用|覆盖目标|写入记忆|取消固定|归档|自动演进/ })).toBeNull();
  });

  it("requires disclosure confirmation, updates consent, and safely schedules reassessment", async () => {
    const user = userEvent.setup();
    renderAssessment();
    const enable = await screen.findByRole("button", { name: "启用评估" });
    expect((enable as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("checkbox", { name: /披露版本 assessment-disclosure-v1/ }));
    expect((enable as HTMLButtonElement).disabled).toBe(false);
    await user.click(enable);
    await waitFor(() => expect(service.updateSkillEvolutionAssessmentConsent).toHaveBeenCalledWith({ enabled: true, evaluatorPolicyVersion: "structured-evaluator-v1", disclosureVersion: "assessment-disclosure-v1" }));
    await user.click(screen.getByRole("button", { name: "请求重新评估" }));
    await waitFor(() => expect(service.scheduleSkillEvolutionReassessment).toHaveBeenCalledWith({ seedId: "seed-1" }));
    expect(await screen.findByText("重新评估已排队。")).toBeTruthy();
    expect(screen.getByText("可进入后续治理")).toBeTruthy();
  });

  it("shows empty and retryable error states without fabricated results", async () => {
    service.querySkillEvolutionAssessments.mockResolvedValueOnce({ items: [] });
    const view = renderAssessment();
    expect(await screen.findByText("暂无评估")).toBeTruthy();
    expect(screen.queryByText("系统置信度")).toBeNull();
    view.unmount();
    service.querySkillEvolutionAssessments.mockRejectedValueOnce(new Error("storage unavailable"));
    renderAssessment();
    expect((await screen.findByRole("alert")).textContent).toContain("无法加载评估");
    expect(screen.getByRole("button", { name: "重试" })).toBeTruthy();
  });
});
