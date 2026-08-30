import { beforeEach, describe, expect, it } from "vitest";
import {
  resetWebSkillGenerationForTest,
  seedWebGenerationJobForTest,
  webSkillGenerationClient,
} from "./web-skill-generation-client";

describe("Skill generation Web adapter contract", () => {
  beforeEach(() => resetWebSkillGenerationForTest());

  it("keeps consent default-off and requires a ready structured provider", async () => {
    const initial = await webSkillGenerationClient.getGenerationPolicy("workspace-one");
    expect(initial.enabled).toBe(false);
    await expect(webSkillGenerationClient.updateGenerationPolicy({
      workspaceId: "workspace-one",
      expectedRevision: initial.revision,
      enabled: true,
      disclosureVersion: initial.disclosureVersion,
      allowedArtifactKinds: initial.allowedArtifactKinds,
    })).rejects.toThrow("generation-provider-not-ready");
    const enabled = await webSkillGenerationClient.updateGenerationPolicy({
      workspaceId: "workspace-one",
      expectedRevision: initial.revision,
      enabled: true,
      disclosureVersion: initial.disclosureVersion,
      providerProfileId: "profile-one",
      modelId: "model-one",
      allowedArtifactKinds: initial.allowedArtifactKinds,
    });
    expect(enabled.revision).toBe(2);
    await expect(webSkillGenerationClient.updateGenerationPolicy({
      ...enabled,
      expectedRevision: enabled.revision - 1,
    })).rejects.toThrow("generation-policy-conflict");
    await expect(webSkillGenerationClient.updateGenerationPolicy({
      ...enabled,
      expectedRevision: enabled.revision,
      allowedArtifactKinds: [],
    })).rejects.toThrow("generation-invalid-request");
  });

  it("models seven stages, bounded pagination, cancellation, and linked regeneration", async () => {
    const running = seedWebGenerationJobForTest("seeded-running");
    seedWebGenerationJobForTest("pagination-completed");
    expect(running.stages).toHaveLength(7);
    const page = await webSkillGenerationClient.listGenerationJobs({ limit: 1 });
    expect(page.items).toHaveLength(1);
    expect(page.nextCursor).toBe("1");
    expect((await webSkillGenerationClient.listGenerationJobs({ limit: 1, cursor: page.nextCursor })).items)
      .toHaveLength(1);
    await expect(webSkillGenerationClient.listGenerationJobs({ limit: 101 }))
      .rejects.toThrow("generation-invalid-request");
    const cancelled = await webSkillGenerationClient.cancelGenerationJob(running.jobId);
    expect(cancelled.status).toBe("cancelled");
    const regenerated = await webSkillGenerationClient.regenerateGenerationJob({
      jobId: running.jobId,
      expectedInputWitnessHash: `mock-input-witness:${running.jobId}`,
      requestId: "regenerated-repair",
    });
    expect(regenerated.supersedesJobId).toBe(running.jobId);
    expect(regenerated.usage.validationRepairs).toBe(1);
    expect((await webSkillGenerationClient.getGenerationJob(running.jobId))?.status).toBe("superseded");
    await expect(webSkillGenerationClient.cancelGenerationJob(running.jobId))
      .rejects.toThrow("generation-job-immutable");
  });

  it("returns bounded dossier/provenance, export disclosure, quarantine, and handoff", async () => {
    seedWebGenerationJobForTest("completed-one");
    const newSkill = seedWebGenerationJobForTest("completed-two");
    expect(newSkill.artifactKind).toBe("new_skill");
    const section = await webSkillGenerationClient.getGenerationDossierSection(
      newSkill.dossierId ?? "",
      0,
      undefined,
      2,
    );
    expect(section.records).toHaveLength(2);
    expect(section.nextCursor).toBe("2");
    expect(JSON.stringify(section).length).toBeLessThan(16_384);
    await expect(webSkillGenerationClient.getGenerationDossierSection(
      newSkill.dossierId ?? "", 13,
    )).rejects.toThrow("generation-invalid-dossier-query");
    const provenance = await webSkillGenerationClient.getGenerationProvenance(newSkill.jobId);
    expect(provenance.modelCalls[0]).not.toHaveProperty("rawPrompt");
    const quarantine = await webSkillGenerationClient.listGenerationQuarantine({ limit: 10 });
    expect(quarantine.items.some((item) => item.jobId === newSkill.jobId)).toBe(true);
    const exported = await webSkillGenerationClient.exportGenerationDossier({
      dossierId: newSkill.dossierId ?? "",
      format: "json",
    });
    expect(exported.exportedFileRemainsUserManaged).toBe(true);
    const handedOff = await webSkillGenerationClient.handoffGenerationPackage(newSkill.jobId);
    expect(handedOff.handoffStatus).toBe("delivered");
    expect(handedOff.permanentlyManual).toBe(true);
  });
});
