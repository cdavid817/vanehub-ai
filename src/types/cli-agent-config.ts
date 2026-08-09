export const cliConfigAgentIds = [
  "claude-code",
  "opencode",
  "codex-cli",
  "antigravity-cli",
] as const;

export type CliConfigAgentId = (typeof cliConfigAgentIds)[number];
export function isCliConfigAgentId(value: string): value is CliConfigAgentId {
  return cliConfigAgentIds.some((candidate) => candidate === value);
}
export type CliConfigDriftState = "detached" | "applied" | "drifted" | "malformed" | "missing";
export type CliConfigValidationState = "valid" | "needs-credential" | "invalid";
export type CliConfigAppliedState = "saved" | "applied" | "drifted";
export type CliConfigDriftResolution = "import-current" | "discard";
export type CliConfigPresetCategory = "official" | "common" | "custom";

export interface ClaudeCodeConfigPayload {
  kind: "claude-code";
  baseUrl: string;
  authMode: "auth-token" | "api-key" | "none";
  model: string;
  haikuModel: string;
  sonnetModel: string;
  opusModel: string;
  advancedEnv: Record<string, string>;
}

export interface CodexCliConfigPayload {
  kind: "codex-cli";
  providerId: string;
  baseUrl: string;
  model: string;
  wireApi: "responses" | "chat";
  reasoningEffort: "none" | "low" | "medium" | "high" | "xhigh" | "max";
  authStrategy: "preserve-official" | "bearer-token" | "replace-auth";
  advancedToml: Record<string, string | number | boolean>;
}

export interface OpenCodeModelDefinition {
  id: string;
  name: string;
}

export interface OpenCodeConfigPayload {
  kind: "opencode";
  providerId: string;
  providerName: string;
  npm: string;
  baseUrl: string;
  headers: Record<string, string>;
  models: OpenCodeModelDefinition[];
  defaultModel: string;
}

export type AntigravityToolPermission =
  | "request-review"
  | "proceed-in-sandbox"
  | "always-proceed"
  | "strict";

/**
 * Antigravity CLI authenticates through the OS keyring with Google Sign-In and speaks a
 * Google-proprietary protocol, so this payload carries neither a credential nor an endpoint:
 * it manages the settings the CLI actually honors.
 */
export interface AntigravityConfigPayload {
  kind: "antigravity";
  toolPermission: AntigravityToolPermission;
  enableTerminalSandbox: boolean;
  verbosity: string;
  model: string;
  advancedSettings: Record<string, string | number | boolean>;
}

export type CliConfigPayload =
  | ClaudeCodeConfigPayload
  | CodexCliConfigPayload
  | OpenCodeConfigPayload
  | AntigravityConfigPayload;

/**
 * Capability declarations, so credential fields, validation actions, and the `needs-credential`
 * state derive from the payload kind rather than from scattered Agent-id conditionals.
 */
export function payloadSupportsCredential(payload: CliConfigPayload): boolean {
  return payload.kind !== "antigravity";
}

/** Payload kinds that can point their CLI at a user-chosen provider endpoint. */
export type EndpointCapableConfigPayload = Exclude<CliConfigPayload, AntigravityConfigPayload>;

/**
 * A type predicate rather than a plain boolean, so the declaration narrows the union too — a
 * caller cannot reach for `baseUrl` on a kind that has none without the compiler objecting.
 */
export function payloadSupportsEndpointOverride(
  payload: CliConfigPayload,
): payload is EndpointCapableConfigPayload {
  return payload.kind !== "antigravity";
}

/**
 * What a kind without a provider endpoint shows in place of one: the local document its profile
 * manages. Kept beside the capability declaration so the two cannot drift apart.
 */
export function managedSettingsPath(payload: CliConfigPayload): string {
  return payload.kind === "antigravity" ? "~/.gemini/antigravity-cli/settings.json" : "";
}

export interface CliConfigPreset {
  id: string;
  catalogVersion: number;
  displayName: string;
  description: string;
  category: CliConfigPresetCategory;
  agentId: CliConfigAgentId;
  deprecated: boolean;
  providerId: string;
  endpointType: import("./agent").ProviderEndpointType;
  iconKey: string;
  payload: CliConfigPayload;
}

export interface CliConfigProfile {
  id: string;
  agentId: CliConfigAgentId;
  name: string;
  payloadVersion: number;
  payload: CliConfigPayload;
  sourcePresetId: string | null;
  sourcePresetVersion: number | null;
  credentialConfigured: boolean;
  validationState: CliConfigValidationState;
  appliedState: CliConfigAppliedState;
  createdAt: string;
  updatedAt: string;
}

export interface SaveCliConfigProfileInput {
  id?: string | null;
  agentId: CliConfigAgentId;
  name: string;
  payload: CliConfigPayload;
  sourcePresetId?: string | null;
  sourcePresetVersion?: number | null;
  credential?: string | null;
  removeCredential?: boolean;
}

export interface ValidateCliConfigCredentialInput {
  agentId: CliConfigAgentId;
  profileId?: string | null;
  payload?: CliConfigPayload | null;
  sourcePresetId?: string | null;
  credential?: string | null;
}

export interface ImportCliConfigProfileInput {
  agentId: CliConfigAgentId;
  name: string;
}

export type CliConfigDiscoveryState = "available" | "unavailable" | "parse-error";

export interface CliConfigDiscoveryCandidate {
  candidateKey: string;
  suggestedName: string;
  providerName: string;
  endpoint: string;
  model: string;
  credentialDetected: boolean;
  isDefault: boolean;
  resolvedPaths: string[];
}

export interface CliConfigDiscoveryResult {
  agentId: CliConfigAgentId;
  state: CliConfigDiscoveryState;
  candidates: CliConfigDiscoveryCandidate[];
  resolvedPaths: string[];
  warnings: string[];
  error: string | null;
  simulated: boolean;
}

export interface ImportDiscoveredCliConfigInput {
  agentId: CliConfigAgentId;
  candidateKeys: string[];
}

export interface SkippedCliConfigDiscoveryCandidate {
  candidateKey: string;
  suggestedName: string;
  reason: "name-conflict";
}

export interface ImportDiscoveredCliConfigResult {
  imported: CliConfigProfile[];
  skipped: SkippedCliConfigDiscoveryCandidate[];
}

export interface DeleteCliConfigProfileInput {
  agentId: CliConfigAgentId;
  profileId: string;
  detachApplied?: boolean;
}

export interface CliConfigStatus {
  agentId: CliConfigAgentId;
  appliedProfileId: string | null;
  driftState: CliConfigDriftState;
  resolvedPaths: string[];
  lastAppliedAt: string | null;
  simulated: boolean;
  startupSync: CliConfigStartupSyncResult;
}

export type CliConfigStartupSyncState =
  | "pending"
  | "imported"
  | "updated"
  | "unchanged"
  | "skipped"
  | "warning"
  | "unavailable";

export interface CliConfigStartupSyncResult {
  agentId: CliConfigAgentId;
  state: CliConfigStartupSyncState;
  imported: number;
  updated: number;
  skipped: number;
  warnings: string[];
  synchronizedAt: string | null;
  simulated: boolean;
}

export interface ApplyCliConfigProfileInput {
  agentId: CliConfigAgentId;
  profileId: string;
  driftResolution?: CliConfigDriftResolution | null;
  confirmAuthFileReplacement?: boolean;
}

export interface CliConfigApplyResult {
  operationId: string;
  status: "succeeded" | "failed";
  agentId: CliConfigAgentId;
  profileId: string;
  affectedPaths: string[];
  driftResolution: CliConfigDriftResolution | null;
  backfilledProfileId: string | null;
  warnings: string[];
  restartRequired: boolean;
  simulated: boolean;
  restored: boolean;
  error: string | null;
}

export interface CliConfigValidationIssue {
  field: string;
  message: string;
}
