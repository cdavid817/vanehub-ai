import { beforeEach, describe, expect, it } from "vitest";
import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";
import { resetWebSkillAssessmentForTest } from "./web-skill-assessment-client";

describe("skill evolution assessment adapter contract", () => {
  beforeEach(resetWebSkillAssessmentForTest);

  it("keeps native and Web adapters behaviorally complete", () => {
    for (const method of [
      "querySkillEvolutionAssessments",
      "getSkillEvolutionAssessment",
      "getSkillEvolutionAssessmentPolicy",
      "updateSkillEvolutionAssessmentConsent",
      "scheduleSkillEvolutionReassessment",
    ] as const) {
      expect(typeof tauriAgentClient[method]).toBe("function");
      expect(typeof webAgentClient[method]).toBe("function");
    }
  });

  it("provides stable paginated current and superseded history", async () => {
    const first = await webAgentClient.querySkillEvolutionAssessments({
      workspace: "mock://history",
      includeHistory: true,
      limit: 1,
    });
    expect(first.items).toHaveLength(1);
    expect(first.nextCursor).toBe("mock-1");
    const second = await webAgentClient.querySkillEvolutionAssessments({
      workspace: "mock://history",
      includeHistory: true,
      limit: 1,
      cursor: first.nextCursor,
    });
    expect(second.items[0]).toMatchObject({ status: "superseded", isCurrent: false });
  });

  it.each(["deterministic", "model-assisted", "fallback", "ambiguous", "pending", "failed", "superseded"])(
    "exposes a sanitized %s fixture",
    async (scenario) => {
      const page = await webAgentClient.querySkillEvolutionAssessments({ workspace: `mock://${scenario}` });
      const detail = await webAgentClient.getSkillEvolutionAssessment(page.items[0].attemptId);
      expect(detail?.attemptId).toBe(page.items[0].attemptId);
      const serialized = JSON.stringify(detail);
      expect(serialized).not.toMatch(/rawPrompt|providerPayload|apiKey|credential/i);
      if (detail?.status === "completed" || detail?.status === "superseded") {
        expect(detail.checks).toHaveLength(9);
      }
    },
  );

  it("persists versioned consent and coalesces reassessment", async () => {
    expect((await webAgentClient.getSkillEvolutionAssessmentPolicy()).modelEvaluationEnabled).toBe(false);
    await expect(webAgentClient.updateSkillEvolutionAssessmentConsent({
      enabled: true,
      evaluatorPolicyVersion: "stale",
      disclosureVersion: "assessment-disclosure-v1",
    })).rejects.toThrow("stale");
    const enabled = await webAgentClient.updateSkillEvolutionAssessmentConsent({
      enabled: true,
      evaluatorPolicyVersion: "structured-evaluator-v1",
      disclosureVersion: "assessment-disclosure-v1",
    });
    expect(enabled.modelEvaluationEnabled).toBe(true);
    const first = await webAgentClient.scheduleSkillEvolutionReassessment({ seedId: "seed-1" });
    const repeated = await webAgentClient.scheduleSkillEvolutionReassessment({ seedId: "seed-1" });
    expect(first.status).toBe("scheduled");
    expect(repeated).toEqual({ queueId: first.queueId, status: "coalesced" });
  });
});
