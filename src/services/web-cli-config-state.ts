import { nowIso } from "./web-mock-clock";
import { slugify } from "./web-mock-identifiers";
import type {
  CliConfigAgentId,
  CliConfigPayload,
  CliConfigProfile,
  CliConfigStatus,
  SaveCliConfigProfileInput,
} from "../types/cli-agent-config";
import { cliConfigAgentIds } from "../types/cli-agent-config";

// Profiles hold `credentialConfigured` as a boolean only — the credential itself is never stored,
// and nothing here is persisted to browser storage.
let webCliConfigProfiles: CliConfigProfile[] = [];
const webCliConfigStatuses = new Map<CliConfigAgentId, CliConfigStatus>();

export function requireCliConfigAgentId(agentId: string): CliConfigAgentId {
  const matched = cliConfigAgentIds.find((candidate) => candidate === agentId);
  if (!matched) throw new Error(`Unsupported CLI configuration Agent: ${agentId}`);
  return matched;
}

export function cloneCliConfigPayload(payload: CliConfigPayload): CliConfigPayload {
  return structuredClone(payload);
}

export function cliConfigNeedsCredential(payload: CliConfigPayload): boolean {
  if (payload.kind === "claude-code") return payload.authMode !== "none";
  if (payload.kind === "codex-cli") return payload.authStrategy !== "preserve-official";
  if (payload.kind === "gemini-cli") return payload.authStrategy !== "preserve-official";
  return payload.kind !== "antigravity";
}

function validateWebCliConfigInput(input: SaveCliConfigProfileInput) {
  if (input.payload.kind !== input.agentId) throw new Error("Profile payload does not match the selected Agent.");
  if (!input.name.trim() || input.name.trim().length > 80) throw new Error("Profile name is required.");
  const baseUrl = input.payload.baseUrl;
  let parsed: URL;
  try {
    parsed = new URL(baseUrl);
  } catch {
    throw new Error("Base URL must be an absolute HTTP(S) URL.");
  }
  if (!(["http:", "https:"] as const).some((protocol) => protocol === parsed.protocol) || parsed.username || parsed.password || parsed.hash) {
    throw new Error("Base URL must be HTTP(S) without credentials or fragments.");
  }
  if (input.payload.kind === "claude-code" && !input.payload.model.trim()) throw new Error("Model is required.");
  if (input.payload.kind === "codex-cli" && (!input.payload.providerId.trim() || !input.payload.model.trim())) {
    throw new Error("Provider and model are required.");
  }
  if (input.payload.kind === "gemini-cli" && !input.payload.model.trim()) throw new Error("Model is required.");
  if (input.payload.kind === "opencode") {
    if (!input.payload.providerId.trim() || input.payload.models.length === 0) throw new Error("Provider and models are required.");
    const defaultModel = input.payload.defaultModel;
    if (!input.payload.models.some((model) => model.id === defaultModel)) {
      throw new Error("Default model must exist in the model list.");
    }
  }
  if (input.sourcePresetId && !input.sourcePresetId.startsWith(`${input.agentId}-`)) {
    throw new Error("Preset is incompatible with the selected Agent.");
  }
}

export function defaultWebCliConfigStatus(agentId: CliConfigAgentId): CliConfigStatus {
  return {
    agentId,
    appliedProfileId: null,
    driftState: "detached",
    resolvedPaths: [],
    lastAppliedAt: null,
    simulated: true,
    startupSync: {
      agentId,
      state: "unavailable",
      imported: 0,
      updated: 0,
      skipped: 0,
      warnings: [],
      synchronizedAt: null,
      simulated: true,
    },
  };
}

export function webCliConfigStatus(agentId: CliConfigAgentId): CliConfigStatus {
  return webCliConfigStatuses.get(agentId) ?? defaultWebCliConfigStatus(agentId);
}

export function setWebCliConfigStatus(agentId: CliConfigAgentId, status: CliConfigStatus) {
  webCliConfigStatuses.set(agentId, status);
}

export function listWebCliConfigProfiles(): CliConfigProfile[] {
  return webCliConfigProfiles;
}

/** The composition root keeps `applyCliConfigProfile`, so it looks a profile up through behavior
 * rather than importing the binding. */
export function findWebCliConfigProfile(agentId: CliConfigAgentId, profileId: string) {
  return webCliConfigProfiles.find(
    (candidate) => candidate.agentId === agentId && candidate.id === profileId,
  );
}

export function removeWebCliConfigProfile(profileId: string) {
  webCliConfigProfiles = webCliConfigProfiles.filter((profile) => profile.id !== profileId);
}

function nextWebCliConfigProfileId(name: string): string {
  const base = slugify(name) || "profile";
  if (!webCliConfigProfiles.some((profile) => profile.id === base)) return base;
  let suffix = 2;
  while (webCliConfigProfiles.some((profile) => profile.id === `${base}-${suffix}`)) suffix += 1;
  return `${base}-${suffix}`;
}

export function saveWebCliConfigProfile(input: SaveCliConfigProfileInput): CliConfigProfile {
  validateWebCliConfigInput(input);
  const existing = input.id
    ? webCliConfigProfiles.find((profile) => profile.id === input.id && profile.agentId === input.agentId)
    : undefined;
  if (input.id && !existing) throw new Error("Profile not found.");
  const credentialConfigured = input.removeCredential
    ? false
    : input.credential
      ? true
      : existing?.credentialConfigured ?? false;
  if (!existing && cliConfigNeedsCredential(input.payload) && !credentialConfigured) {
    throw new Error("Credential repair is required.");
  }
  const timestamp = nowIso();
  let status = webCliConfigStatus(input.agentId);
  if (
    existing
    && status.appliedProfileId === existing.id
    && JSON.stringify(existing.payload) !== JSON.stringify(input.payload)
  ) {
    status = { ...status, driftState: "drifted" };
    webCliConfigStatuses.set(input.agentId, status);
  }
  const profile: CliConfigProfile = {
    id: existing?.id ?? nextWebCliConfigProfileId(input.name),
    agentId: input.agentId,
    name: input.name.trim(),
    payloadVersion: 1,
    payload: cloneCliConfigPayload(input.payload),
    sourcePresetId: input.sourcePresetId ?? existing?.sourcePresetId ?? null,
    sourcePresetVersion: input.sourcePresetVersion ?? existing?.sourcePresetVersion ?? null,
    credentialConfigured,
    validationState: cliConfigNeedsCredential(input.payload) && !credentialConfigured ? "needs-credential" : "valid",
    appliedState:
      status.appliedProfileId === existing?.id
        ? status.driftState === "applied"
          ? "applied"
          : "drifted"
        : "saved",
    createdAt: existing?.createdAt ?? timestamp,
    updatedAt: timestamp,
  };
  webCliConfigProfiles = [...webCliConfigProfiles.filter((item) => item.id !== profile.id), profile];
  return structuredClone(profile);
}
