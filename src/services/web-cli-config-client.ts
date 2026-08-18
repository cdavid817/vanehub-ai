import type { CliConfigService } from "./cli-service";
import {
  cliConfigNeedsCredential,
  cloneCliConfigPayload,
  defaultWebCliConfigStatus,
  listWebCliConfigProfiles,
  removeWebCliConfigProfile,
  requireCliConfigAgentId,
  saveWebCliConfigProfile,
  setWebCliConfigStatus,
  webCliConfigStatus,
} from "./web-cli-config-state";
import { getCliConfigPresets } from "../config/cli-agent-provider-presets";

export const webCliConfigClient: CliConfigService = {
  async listCliConfigPresets(agentId) {
    return getCliConfigPresets(requireCliConfigAgentId(agentId));
  },

  async listCliConfigProfiles(agentId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    const status = webCliConfigStatus(supportedAgentId);
    return listWebCliConfigProfiles()
      .filter((profile) => profile.agentId === supportedAgentId)
      .map((profile) => ({
        ...structuredClone(profile),
        appliedState:
          status.appliedProfileId === profile.id
            ? status.driftState === "applied"
              ? "applied" as const
              : "drifted" as const
            : "saved" as const,
      }));
  },

  async getCliConfigStatus(agentId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    return structuredClone(webCliConfigStatus(supportedAgentId));
  },

  async saveCliConfigProfile(input) {
    return saveWebCliConfigProfile(input);
  },

  async validateCliConfigCredential(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const profile = input.profileId
      ? listWebCliConfigProfiles().find((candidate) => candidate.agentId === supportedAgentId && candidate.id === input.profileId)
      : undefined;
    if (input.profileId && !profile) throw new Error("Profile not found.");
    const payload = input.payload ?? profile?.payload;
    if (!payload || payload.kind !== supportedAgentId) throw new Error("A complete provider configuration is required.");
    if (!cliConfigNeedsCredential(payload)) {
      return { status: "unsupported" as const, latencyMs: 0, httpStatus: null };
    }
    const credential = input.credential?.trim();
    if (!credential && !profile?.credentialConfigured) throw new Error("Credential repair is required.");
    const status = credential === "web-invalid"
      ? "invalid-credential" as const
      : credential === "web-rate-limited"
        ? "rate-limited" as const
        : credential === "web-unavailable"
          ? "provider-unavailable" as const
          : "valid" as const;
    return { status, latencyMs: 12, httpStatus: status === "valid" ? 200 : status === "invalid-credential" ? 401 : status === "rate-limited" ? 429 : null };
  },

  async duplicateCliConfigProfile(agentId, profileId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    const source = listWebCliConfigProfiles().find(
      (profile) => profile.agentId === supportedAgentId && profile.id === profileId,
    );
    if (!source) throw new Error("Profile not found.");
    let name = `${source.name} Copy`;
    let suffix = 2;
    while (listWebCliConfigProfiles().some((profile) => profile.agentId === supportedAgentId && profile.name === name)) {
      name = `${source.name} Copy ${suffix++}`;
    }
    return saveWebCliConfigProfile({
      agentId: supportedAgentId,
      name,
      payload: cloneCliConfigPayload(source.payload),
      sourcePresetId: source.sourcePresetId,
      sourcePresetVersion: source.sourcePresetVersion,
      credential: source.credentialConfigured ? "web-simulated-credential" : null,
    });
  },

  async deleteCliConfigProfile(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const status = webCliConfigStatus(supportedAgentId);
    const exists = listWebCliConfigProfiles().some(
      (profile) => profile.agentId === supportedAgentId && profile.id === input.profileId,
    );
    if (!exists) throw new Error("Profile not found.");
    if (status.appliedProfileId === input.profileId && !input.detachApplied) {
      throw new Error("Applied profile must be detached before deletion.");
    }
    if (status.appliedProfileId === input.profileId) {
      setWebCliConfigStatus(supportedAgentId, defaultWebCliConfigStatus(supportedAgentId));
    }
    removeWebCliConfigProfile(input.profileId);
  },

  async importCliConfigProfile(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const preset = getCliConfigPresets(supportedAgentId)[0];
    if (!preset) throw new Error("No simulated global configuration is available.");
    return saveWebCliConfigProfile({
      agentId: supportedAgentId,
      name: input.name,
      payload: cloneCliConfigPayload(preset.payload),
      sourcePresetId: null,
      sourcePresetVersion: null,
      credential: cliConfigNeedsCredential(preset.payload) ? "web-imported-credential" : null,
    });
  },

  async discoverCliConfigProfiles(agentId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    return {
      agentId: supportedAgentId,
      state: "unavailable" as const,
      candidates: [],
      resolvedPaths: [],
      warnings: [],
      error: null,
      simulated: true,
    };
  },

  async importDiscoveredCliConfigProfiles(input) {
    requireCliConfigAgentId(input.agentId);
    if (input.candidateKeys.length > 0) {
      throw new Error("Local configuration discovery is unavailable in Web mode.");
    }
    return { imported: [], skipped: [] };
  },
};
