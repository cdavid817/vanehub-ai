import { describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";

describe("Web CLI global configuration", () => {
  it("reports local discovery as unavailable without fabricated candidates", async () => {
    await expect(webAgentClient.discoverCliConfigProfiles("opencode")).resolves.toEqual({
      agentId: "opencode",
      state: "unavailable",
      candidates: [],
      resolvedPaths: [],
      warnings: [],
      error: null,
      simulated: true,
    });
    await expect(webAgentClient.importDiscoveredCliConfigProfiles({
      agentId: "opencode",
      candidateKeys: ["openrouter"],
    })).rejects.toThrow("unavailable in Web mode");
  });

  it("exposes only compatible secret-free presets", async () => {
    const presets = await webAgentClient.listCliConfigPresets("claude-code");
    expect(presets.length).toBeGreaterThanOrEqual(8);
    expect(presets.every((preset) => preset.agentId === "claude-code")).toBe(true);
    const serialized = JSON.stringify(presets).toLowerCase();
    expect(serialized).not.toContain("credentialref");
    expect(serialized).not.toContain("authorization\":");
    await expect(webAgentClient.listCliConfigPresets("gemini-cli")).rejects.toThrow("Unsupported");
  });

  it("creates, applies, duplicates, and deletes profiles without changing workflow state", async () => {
    const preset = (await webAgentClient.listCliConfigPresets("claude-code"))[0];
    if (!preset || preset.payload.kind !== "claude-code") throw new Error("preset fixture missing");
    const workflowBefore = await webAgentClient.getWorkflowState();
    const sessionBefore = await webAgentClient.getActiveSession();
    const profile = await webAgentClient.saveCliConfigProfile({
      agentId: "claude-code",
      name: `Official ${Date.now()}`,
      payload: preset.payload,
      sourcePresetId: preset.id,
      sourcePresetVersion: preset.catalogVersion,
    });
    expect(profile.credentialConfigured).toBe(false);

    const applied = await webAgentClient.applyCliConfigProfile({
      agentId: "claude-code",
      profileId: profile.id,
    });
    expect(applied).toMatchObject({ simulated: true, affectedPaths: [], status: "succeeded" });
    expect(await webAgentClient.getWorkflowState()).toEqual(workflowBefore);
    expect(await webAgentClient.getActiveSession()).toEqual(sessionBefore);

    const duplicate = await webAgentClient.duplicateCliConfigProfile("claude-code", profile.id);
    expect(duplicate.id).not.toBe(profile.id);
    await webAgentClient.deleteCliConfigProfile({
      agentId: "claude-code",
      profileId: profile.id,
      detachApplied: true,
    });
    expect((await webAgentClient.listCliConfigProfiles("claude-code")).some((item) => item.id === profile.id)).toBe(false);
  });

  it("discards submitted secrets and reports credential repair after removal", async () => {
    const preset = (await webAgentClient.listCliConfigPresets("codex-cli"))
      .find((candidate) => candidate.displayName === "OpenRouter");
    if (!preset || preset.payload.kind !== "codex-cli") throw new Error("preset fixture missing");
    await expect(webAgentClient.saveCliConfigProfile({
      agentId: "codex-cli",
      name: `Missing ${Date.now()}`,
      payload: preset.payload,
    })).rejects.toThrow("Credential");

    const secret = "sk-do-not-return-this-value";
    const profile = await webAgentClient.saveCliConfigProfile({
      agentId: "codex-cli",
      name: `Credential ${Date.now()}`,
      payload: preset.payload,
      credential: secret,
    });
    expect(JSON.stringify(profile)).not.toContain(secret);
    const repaired = await webAgentClient.saveCliConfigProfile({
      id: profile.id,
      agentId: "codex-cli",
      name: profile.name,
      payload: profile.payload,
      removeCredential: true,
    });
    expect(repaired).toMatchObject({ credentialConfigured: false, validationState: "needs-credential" });
    await expect(webAgentClient.applyCliConfigProfile({
      agentId: "codex-cli",
      profileId: repaired.id,
    })).rejects.toThrow("Credential");
  });

  it("validates transient and stored CLI credentials without retaining the transient value", async () => {
    const preset = (await webAgentClient.listCliConfigPresets("codex-cli"))
      .find((candidate) => candidate.displayName === "OpenRouter");
    if (!preset) throw new Error("preset fixture missing");
    await expect(webAgentClient.validateCliConfigCredential({
      agentId: "codex-cli",
      payload: preset.payload,
      sourcePresetId: preset.id,
      credential: "web-invalid",
    })).resolves.toMatchObject({ status: "invalid-credential", httpStatus: 401 });

    const profile = await webAgentClient.saveCliConfigProfile({
      agentId: "codex-cli",
      name: `Validation ${Date.now()}`,
      payload: preset.payload,
      sourcePresetId: preset.id,
      credential: "stored-secret",
    });
    await expect(webAgentClient.validateCliConfigCredential({ agentId: "codex-cli", profileId: profile.id }))
      .resolves.toMatchObject({ status: "valid", httpStatus: 200 });
    expect(JSON.stringify(profile)).not.toContain("stored-secret");
  });

  it("automatically backfills the leaving exclusive profile without a drift choice", async () => {
    const preset = (await webAgentClient.listCliConfigPresets("claude-code"))[0];
    if (!preset || preset.payload.kind !== "claude-code") throw new Error("preset fixture missing");
    const profile = await webAgentClient.saveCliConfigProfile({
      agentId: "claude-code",
      name: `Drift ${Date.now()}`,
      payload: preset.payload,
    });
    await webAgentClient.applyCliConfigProfile({ agentId: "claude-code", profileId: profile.id });
    await webAgentClient.saveCliConfigProfile({
      id: profile.id,
      agentId: "claude-code",
      name: profile.name,
      payload: { ...preset.payload, model: "claude-edited-outside-projection" },
    });

    expect(await webAgentClient.getCliConfigStatus("claude-code")).toMatchObject({ driftState: "drifted" });
    const target = await webAgentClient.saveCliConfigProfile({
      agentId: "claude-code",
      name: `Target ${Date.now()}`,
      payload: preset.payload,
    });
    await expect(webAgentClient.applyCliConfigProfile({
      agentId: "claude-code",
      profileId: target.id,
    })).resolves.toMatchObject({
      status: "succeeded",
      driftResolution: "import-current",
      backfilledProfileId: profile.id,
    });
  });
});
