import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  resetWebSkillEvolutionOrchestrationForTest,
  seedWebEvolutionBreakerForTest,
  webSkillEvolutionOrchestrationClient,
} from "./web-skill-evolution-orchestration-client";

describe("Skill evolution orchestration Web adapter contract", () => {
  beforeEach(() => resetWebSkillEvolutionOrchestrationForTest());

  it("defaults off and requires current consent plus a stable allowlist", async () => {
    const initial = await webSkillEvolutionOrchestrationClient.getEvolutionPolicy("workspace-one");
    expect(initial).toMatchObject({ mode: "off", mockProvenance: "web_simulation" });
    await expect(webSkillEvolutionOrchestrationClient.updateEvolutionPolicy({
      workspaceId: "workspace-one", expectedRevision: 0, mode: "enabled",
      allowedSkillIds: [], acknowledgeCurrentDisclosure: true,
    })).rejects.toThrow("consent_and_allowlist_required");
    await expect(webSkillEvolutionOrchestrationClient.updateEvolutionPolicy({
      workspaceId: "workspace-one", expectedRevision: 0, mode: "enabled",
      allowedSkillIds: ["*"], acknowledgeCurrentDisclosure: true,
    })).rejects.toThrow("invalid_allowlist");
    const enabled = await webSkillEvolutionOrchestrationClient.updateEvolutionPolicy({
      workspaceId: "workspace-one", expectedRevision: 0, mode: "enabled",
      allowedSkillIds: ["skill-one"], acknowledgeCurrentDisclosure: true,
    });
    expect(enabled.consent?.disclosureVersion).toBe("skill-evolution-orchestration-disclosure-v1");
    await expect(webSkillEvolutionOrchestrationClient.updateEvolutionPolicy({
      workspaceId: "workspace-one", expectedRevision: 0, mode: "off",
      allowedSkillIds: [], acknowledgeCurrentDisclosure: false,
    })).rejects.toThrow("stale_conflict");
  });

  it("simulates observe and enabled outcomes without claiming native effects", async () => {
    const notification = vi.fn();
    const unsubscribe = await webSkillEvolutionOrchestrationClient.subscribeEvolutionNotifications(notification);
    await webSkillEvolutionOrchestrationClient.updateEvolutionPolicy({
      workspaceId: "observe", expectedRevision: 0, mode: "observe",
      allowedSkillIds: ["skill-observed"], acknowledgeCurrentDisclosure: false,
    });
    await webSkillEvolutionOrchestrationClient.requestEvolutionRun("observe");
    const observed = await webSkillEvolutionOrchestrationClient.listEvolutionEligibility({
      workspaceId: "observe",
    });
    expect(observed.items[0]).toMatchObject({
      result: "would_apply", mockProvenance: "web_simulation",
    });
    expect((await webSkillEvolutionOrchestrationClient.listEvolutionApplications({
      workspaceId: "observe",
    })).items).toHaveLength(0);

    await webSkillEvolutionOrchestrationClient.updateEvolutionPolicy({
      workspaceId: "enabled", expectedRevision: 0, mode: "enabled",
      allowedSkillIds: ["skill-enabled"], acknowledgeCurrentDisclosure: true,
    });
    await webSkillEvolutionOrchestrationClient.requestEvolutionRun("enabled");
    const application = await webSkillEvolutionOrchestrationClient.listEvolutionApplications({
      workspaceId: "enabled",
    });
    const probation = await webSkillEvolutionOrchestrationClient.listEvolutionProbations({
      workspaceId: "enabled",
    });
    expect(application.items[0].mockProvenance).toBe("web_simulation");
    expect(application.items[0].overlayApplicationId).toContain("simulation");
    expect(probation.items[0]).toMatchObject({ status: "active", mockProvenance: "web_simulation" });
    expect(notification).toHaveBeenCalledWith(expect.objectContaining({
      eventKind: "automatic_application", mockProvenance: "web_simulation",
    }));
    unsubscribe();
  });

  it("is page-scoped, bounds queries, and gates breaker acknowledgement", async () => {
    const breaker = seedWebEvolutionBreakerForTest("workspace-one");
    const closed = await webSkillEvolutionOrchestrationClient.acknowledgeEvolutionBreaker(
      breaker.breakerId,
      breaker.revision,
    );
    expect(closed.status).toBe("closed");
    await expect(webSkillEvolutionOrchestrationClient.listEvolutionRuns({
      workspaceId: "workspace-one", cursor: "invalid",
    })).rejects.toThrow("invalid_input");
    resetWebSkillEvolutionOrchestrationForTest();
    expect((await webSkillEvolutionOrchestrationClient.listEvolutionBreakers({
      workspaceId: "workspace-one",
    })).items).toHaveLength(0);
  });
});
