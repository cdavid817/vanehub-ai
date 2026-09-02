import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import { formatDiagnosticSummary } from "../../../ui/diagnostics/diagnostic-field";
import type { CliConfigPayload, CliConfigProfile, CliConfigStatus } from "../../../types/cli-agent-config";
import type { OnePieceProviderProfile, OnePieceProviderProfiles } from "../../../types/agent";
import { buildCliAgentConfigDiagnosticFields, buildOnePieceConfigDiagnosticFields } from "./agent-configuration-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function prefixed(profileId: string, fieldKey: string): string {
  return t("agentConfigurations.diagnostics.field.profilePrefixed", { profile: profileId, field: t(`agentConfigurations.diagnostics.field.${fieldKey}`) });
}

function cliStatus(overrides: Partial<CliConfigStatus> = {}): CliConfigStatus {
  return {
    agentId: "claude-code",
    appliedProfileId: null,
    driftState: "detached",
    resolvedPaths: [],
    lastAppliedAt: null,
    simulated: true,
    startupSync: {
      agentId: "claude-code",
      state: "unavailable",
      imported: 0,
      updated: 0,
      skipped: 0,
      warnings: [],
      synchronizedAt: null,
      simulated: true,
    },
    ...overrides,
  };
}

function claudeCodePayload(overrides: Partial<Extract<CliConfigPayload, { kind: "claude-code" }>> = {}): CliConfigPayload {
  return {
    kind: "claude-code",
    baseUrl: "https://api.anthropic.com",
    authMode: "api-key",
    model: "claude-sonnet-4",
    haikuModel: "",
    sonnetModel: "",
    opusModel: "",
    advancedEnv: {},
    ...overrides,
  };
}

function cliProfile(overrides: Partial<CliConfigProfile> = {}): CliConfigProfile {
  return {
    id: "profile-1",
    agentId: "claude-code",
    name: "My profile",
    payloadVersion: 1,
    payload: claudeCodePayload(),
    sourcePresetId: "preset-1",
    sourcePresetVersion: 2,
    credentialConfigured: true,
    validationState: "valid",
    appliedState: "applied",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-02-01T00:00:00Z",
    ...overrides,
  };
}

function onePieceProfile(overrides: Partial<OnePieceProviderProfile> = {}): OnePieceProviderProfile {
  return {
    id: "op-1",
    name: "My OnePiece profile",
    sourceProviderId: "preset-anthropic",
    sourceEndpointType: "anthropic-messages",
    sourcePresetVersion: 3,
    provider: "Anthropic",
    modelId: "claude-sonnet-4",
    interfaceFormat: "anthropic",
    baseUrl: "https://api.anthropic.com",
    active: true,
    credentialPresent: true,
    ...overrides,
  };
}

const SECRET_NAME = "Prod key (do not share)";
const SECRET_ENV_VALUE = "sk-ant-api03-leaked-from-advanced-env";
const SECRET_API_KEY = "sk-ant-api03-do-not-leak-this-value";

describe("buildCliAgentConfigDiagnosticFields", () => {
  it("never includes a profile's own free-text name", () => {
    const fields = buildCliAgentConfigDiagnosticFields("claude-code", cliStatus(), [cliProfile({ name: SECRET_NAME })], t);
    const summary = formatDiagnosticSummary(fields, "unavailable");
    expect(summary).not.toContain(SECRET_NAME);
    expect(fields.some((field) => field.label.includes(SECRET_NAME))).toBe(false);
  });

  it("never leaks a secret planted inside a payload's own free-form advancedEnv record", () => {
    const fields = buildCliAgentConfigDiagnosticFields(
      "claude-code",
      cliStatus(),
      [cliProfile({ payload: claudeCodePayload({ advancedEnv: { ANTHROPIC_AUTH_TOKEN: SECRET_ENV_VALUE } }) })],
      t,
    );
    expect(formatDiagnosticSummary(fields, "unavailable")).not.toContain(SECRET_ENV_VALUE);
  });

  it("reports credentialConfigured as a boolean flag, never a credential value", () => {
    const withCredential = buildCliAgentConfigDiagnosticFields("claude-code", cliStatus(), [cliProfile({ credentialConfigured: true })], t);
    expect(new Map(withCredential.map((f) => [f.label, f.value])).get(prefixed("profile-1", "credentialConfigured"))).toBe("true");

    const withoutCredential = buildCliAgentConfigDiagnosticFields("claude-code", cliStatus(), [cliProfile({ credentialConfigured: false })], t);
    expect(new Map(withoutCredential.map((f) => [f.label, f.value])).get(prefixed("profile-1", "credentialConfigured"))).toBe("false");
  });

  it("prefixes each profile's own fields with its stable id, not its free-text name", () => {
    const fields = buildCliAgentConfigDiagnosticFields("codex-cli", cliStatus(), [cliProfile({ id: "profile-42", name: "Whatever I typed" })], t);
    expect(fields.some((field) => field.label === prefixed("profile-42", "payloadVersion"))).toBe(true);
    expect(fields.some((field) => field.label.startsWith("Whatever I typed"))).toBe(false);
  });

  it("includes drift state, joined resolved paths, and raw ISO timestamps from the status query", () => {
    const status = cliStatus({
      driftState: "drifted",
      appliedProfileId: "profile-1",
      resolvedPaths: ["~/.claude/settings.json", "~/.claude.json"],
      lastAppliedAt: "2026-03-01T00:00:00Z",
      startupSync: { agentId: "claude-code", state: "updated", imported: 2, updated: 1, skipped: 0, warnings: ["ignored"], synchronizedAt: "2026-03-01T00:05:00Z", simulated: false },
    });
    const byLabel = new Map(buildCliAgentConfigDiagnosticFields("claude-code", status, [], t).map((f) => [f.label, f.value]));
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.driftState"))).toBe("drifted");
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.appliedProfileId"))).toBe("profile-1");
    expect(byLabel.get(t("agentConfigurations.status.paths"))).toBe("~/.claude/settings.json, ~/.claude.json");
    expect(byLabel.get(t("agentConfigurations.status.lastApplied"))).toBe("2026-03-01T00:00:00Z");
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.startupSyncState"))).toBe("updated");
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.startupSyncImported"))).toBe("2");
  });

  it("marks status fields unavailable rather than guessing before the status query has loaded", () => {
    const byLabel = new Map(buildCliAgentConfigDiagnosticFields("claude-code", undefined, [], t).map((f) => [f.label, f.value]));
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.driftState"))).toBeNull();
    expect(byLabel.get(t("agentConfigurations.status.paths"))).toBeNull();
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.simulated"))).toBeNull();
  });

  it("reports each payload kind's own bounded auth fields, keyed to that kind alone", () => {
    const codexFields = buildCliAgentConfigDiagnosticFields("codex-cli", cliStatus(), [cliProfile({
      payload: { kind: "codex-cli", providerId: "custom", baseUrl: "https://api.example.com", model: "gpt-5", wireApi: "responses", reasoningEffort: "high", authStrategy: "bearer-token", advancedToml: {} },
    })], t);
    const codexByLabel = new Map(codexFields.map((f) => [f.label, f.value]));
    expect(codexByLabel.get(prefixed("profile-1", "wireApi"))).toBe("responses");
    expect(codexByLabel.get(prefixed("profile-1", "reasoningEffort"))).toBe("high");
    expect(codexByLabel.get(prefixed("profile-1", "authStrategy"))).toBe("bearer-token");

    const antigravityFields = buildCliAgentConfigDiagnosticFields("antigravity-cli", cliStatus(), [cliProfile({
      payload: { kind: "antigravity", toolPermission: "strict", enableTerminalSandbox: true, verbosity: "debug", model: "gemini-3-pro", advancedSettings: {} },
    })], t);
    const antigravityByLabel = new Map(antigravityFields.map((f) => [f.label, f.value]));
    expect(antigravityByLabel.get(prefixed("profile-1", "toolPermission"))).toBe("strict");
    expect(antigravityByLabel.get(prefixed("profile-1", "terminalSandboxEnabled"))).toBe("true");
    // Antigravity has no custom endpoint; the managed settings path stands in for it.
    expect(antigravityByLabel.get(prefixed("profile-1", "endpoint"))).toBe("~/.gemini/antigravity-cli/settings.json");
    // No free-typed `verbosity` field anywhere in the output -- excluded as user-typed free text.
    expect(antigravityFields.some((f) => f.value === "debug")).toBe(false);

    const openCodeFields = buildCliAgentConfigDiagnosticFields("opencode", cliStatus(), [cliProfile({
      payload: { kind: "opencode", providerId: "custom", providerName: "My Provider", npm: "@ai-sdk/openai-compatible", baseUrl: "https://api.example.com", headers: {}, models: [], defaultModel: "gpt-5" },
    })], t);
    // opencode has no bounded auth/permission enum of its own beyond endpoint + model.
    expect(openCodeFields.some((f) => f.label === prefixed("profile-1", "authMode"))).toBe(false);
    expect(new Map(openCodeFields.map((f) => [f.label, f.value])).get(prefixed("profile-1", "model"))).toBe("gpt-5");
  });

  it("never carries anything beyond the bounded fields this snapshot type can hold", () => {
    const fields = buildCliAgentConfigDiagnosticFields("claude-code", cliStatus(), [cliProfile()], t);
    expect(fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string"))).toBe(true);
  });
});

describe("buildOnePieceConfigDiagnosticFields (redaction)", () => {
  it("never includes a planted apiKey-shaped value anywhere in the formatted output", () => {
    // `OnePieceProviderProfile` has no apiKey field at all -- confirmed write-only by reading every
    // mutation input type that carries it. This fixture defends against the shape ever drifting (a
    // backend response that started including it anyway), the same posture
    // im-diagnostic-summary.test.ts's own SECRET_VALUE fixture takes for `publicConfig`.
    const leaked = { ...onePieceProfile(), apiKey: SECRET_API_KEY } as unknown as OnePieceProviderProfile;
    const overview: OnePieceProviderProfiles = { profiles: [leaked], activeProfileId: leaked.id };
    const summary = formatDiagnosticSummary(buildOnePieceConfigDiagnosticFields(overview, t), "unavailable");
    expect(summary).not.toContain(SECRET_API_KEY);
  });

  it("reports credentialPresent as the shared credentialConfigured boolean flag, never the key", () => {
    const withKey = buildOnePieceConfigDiagnosticFields({ profiles: [onePieceProfile({ credentialPresent: true })], activeProfileId: "op-1" }, t);
    expect(new Map(withKey.map((f) => [f.label, f.value])).get(prefixed("op-1", "credentialConfigured"))).toBe("true");

    const withoutKey = buildOnePieceConfigDiagnosticFields({ profiles: [onePieceProfile({ credentialPresent: false })], activeProfileId: null }, t);
    expect(new Map(withoutKey.map((f) => [f.label, f.value])).get(prefixed("op-1", "credentialConfigured"))).toBe("false");
  });

  it("never includes the profile's own free-text name", () => {
    const overview: OnePieceProviderProfiles = { profiles: [onePieceProfile({ name: SECRET_NAME })], activeProfileId: "op-1" };
    const fields = buildOnePieceConfigDiagnosticFields(overview, t);
    const summary = formatDiagnosticSummary(fields, "unavailable");
    expect(summary).not.toContain(SECRET_NAME);
    expect(fields.some((field) => field.label.includes(SECRET_NAME))).toBe(false);
  });

  it("includes the catalog-derived provider, model, endpoint, and format as raw backend values", () => {
    const overview: OnePieceProviderProfiles = {
      profiles: [onePieceProfile({ id: "op-9", provider: "OpenRouter", modelId: "gpt-5", interfaceFormat: "openai-compatible", baseUrl: "https://openrouter.ai/api/v1", active: false })],
      activeProfileId: null,
    };
    const byLabel = new Map(buildOnePieceConfigDiagnosticFields(overview, t).map((f) => [f.label, f.value]));
    expect(byLabel.get(prefixed("op-9", "provider"))).toBe("OpenRouter");
    expect(byLabel.get(prefixed("op-9", "model"))).toBe("gpt-5");
    expect(byLabel.get(prefixed("op-9", "interfaceFormat"))).toBe("openai-compatible");
    expect(byLabel.get(prefixed("op-9", "endpoint"))).toBe("https://openrouter.ai/api/v1");
    expect(byLabel.get(prefixed("op-9", "active"))).toBe("false");
  });

  it("includes the page-level agentId and activeProfileId, marking activeProfileId unavailable when nothing is active", () => {
    const byLabel = new Map(buildOnePieceConfigDiagnosticFields({ profiles: [], activeProfileId: null }, t).map((f) => [f.label, f.value]));
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.agentId"))).toBe("onepiece");
    expect(byLabel.get(t("agentConfigurations.diagnostics.field.activeProfileId"))).toBeNull();
  });

  it("handles an undefined overview (query not yet settled) without throwing", () => {
    expect(() => buildOnePieceConfigDiagnosticFields(undefined, t)).not.toThrow();
    const fields = buildOnePieceConfigDiagnosticFields(undefined, t);
    expect(fields.some((field) => field.label === t("agentConfigurations.diagnostics.field.activeProfileId") && field.value === null)).toBe(true);
  });

  it("never carries anything beyond the bounded fields this profile type can hold", () => {
    const overview: OnePieceProviderProfiles = { profiles: [onePieceProfile(), onePieceProfile({ id: "op-2", sourceProviderId: null, sourceEndpointType: null, sourcePresetVersion: null, baseUrl: null })], activeProfileId: "op-1" };
    const fields = buildOnePieceConfigDiagnosticFields(overview, t);
    expect(fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string"))).toBe(true);
  });
});
