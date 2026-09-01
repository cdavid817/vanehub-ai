import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../i18n";
import { formatDiagnosticSummary } from "../../ui/diagnostics/diagnostic-field";
import type { ExecutionObservationCapability, ObservabilitySettings } from "../../types/execution-observability";
import { buildObservabilityDiagnosticFields } from "./observability-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function settings(overrides: Partial<ObservabilitySettings> = {}): ObservabilitySettings {
  return {
    localTimelineEnabled: true,
    otlpEnabled: false,
    otlpEndpoint: null,
    otlpProtocol: "http_protobuf",
    samplingRatio: 1,
    retentionDays: 30,
    capturePolicy: "metadata_only",
    mcpRelayEnabled: false,
    otlpAuthConfigured: false,
    ...overrides,
  };
}

function capability(overrides: Partial<ExecutionObservationCapability> = {}): ExecutionObservationCapability {
  return {
    agentId: "claude-code",
    transport: "stdio",
    toolFidelity: "inferred",
    mcpFidelity: "opaque",
    relaySupported: false,
    detail: "No safe invocation-scoped MCP configuration is verified",
    ...overrides,
  };
}

// A realistic-looking bearer token, playing the same "value worth protecting" role SSH's
// SECRET_HOST/IM's SECRET_VALUE constants do -- `ObservabilitySettings.otlpAuthToken` is a real
// (write-only) field this page's own data model has, even though the builder never reads it.
const SECRET_TOKEN = "sk-live-9f2a7c31e8b4406fa2b0e6d4c9a71234";

describe("buildObservabilityDiagnosticFields (redaction)", () => {
  it("never includes the OTLP auth token, even when the settings object passed in carries one", () => {
    const fields = buildObservabilityDiagnosticFields(
      settings({ otlpAuthToken: SECRET_TOKEN, otlpAuthConfigured: true }),
      [],
      t,
    );
    const summary = formatDiagnosticSummary(fields, "unavailable");
    expect(summary).not.toContain(SECRET_TOKEN);
    expect(fields.every((field) => field.value !== SECRET_TOKEN)).toBe(true);
  });

  it("reports otlpAuthConfigured as a boolean flag, never a token value", () => {
    const configured = buildObservabilityDiagnosticFields(settings({ otlpAuthConfigured: true }), [], t);
    const byLabelTrue = new Map(configured.map((field) => [field.label, field.value]));
    expect(byLabelTrue.get(t("observability.diagnostics.field.otlpAuthConfigured"))).toBe("true");

    const notConfigured = buildObservabilityDiagnosticFields(settings({ otlpAuthConfigured: false }), [], t);
    const byLabelFalse = new Map(notConfigured.map((field) => [field.label, field.value]));
    expect(byLabelFalse.get(t("observability.diagnostics.field.otlpAuthConfigured"))).toBe("false");
  });

  it("includes the settings' own safe fields as raw backend values", () => {
    const fields = buildObservabilityDiagnosticFields(
      settings({
        localTimelineEnabled: false,
        retentionDays: 14,
        otlpEnabled: true,
        otlpEndpoint: "https://collector.example.com/v1/traces",
        samplingRatio: 0.5,
        capturePolicy: "redacted_content",
        mcpRelayEnabled: true,
      }),
      [],
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("observability.local.enabled"))).toBe("false");
    expect(byLabel.get(t("observability.retention"))).toBe("14");
    expect(byLabel.get(t("observability.export.enabled"))).toBe("true");
    expect(byLabel.get(t("observability.export.endpoint"))).toBe("https://collector.example.com/v1/traces");
    expect(byLabel.get(t("observability.export.sampling"))).toBe("0.5");
    expect(byLabel.get(t("observability.capture.policy"))).toBe("redacted_content");
    expect(byLabel.get(t("observability.mcp.relay"))).toBe("true");
  });

  it("marks the OTLP endpoint unavailable rather than guessing when export is off", () => {
    const fields = buildObservabilityDiagnosticFields(settings({ otlpEndpoint: null }), [], t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("observability.export.endpoint"))).toBeNull();
  });

  it("summarizes MCP relay capability per stdio agent, mirroring the page's own rendered grid", () => {
    const capabilities: ExecutionObservationCapability[] = [
      capability({ agentId: "claude-code", transport: "stdio", relaySupported: true }),
      capability({ agentId: "claude-code", transport: "http", relaySupported: true }),
      capability({ agentId: "codex-cli", transport: "stdio", relaySupported: true }),
      capability({ agentId: "gemini-cli", transport: "stdio", relaySupported: false }),
      capability({ agentId: "opencode", transport: "http", relaySupported: false }),
    ];
    const fields = buildObservabilityDiagnosticFields(settings(), capabilities, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("observability.diagnostics.field.relayAvailable"))).toBe("true");
    expect(byLabel.get(t("observability.diagnostics.field.relaySupportedAgentIds"))).toBe("claude-code, codex-cli");
    expect(byLabel.get(t("observability.diagnostics.field.opaqueAgentIds"))).toBe("gemini-cli");
  });

  it("reports relayAvailable as false and both agent lists unavailable when there are no capabilities", () => {
    const fields = buildObservabilityDiagnosticFields(settings(), [], t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("observability.diagnostics.field.relayAvailable"))).toBe("false");
    expect(byLabel.get(t("observability.diagnostics.field.relaySupportedAgentIds"))).toBeNull();
    expect(byLabel.get(t("observability.diagnostics.field.opaqueAgentIds"))).toBeNull();
  });

  it("never carries anything beyond the bounded fields this page's data model can hold", () => {
    const fields = buildObservabilityDiagnosticFields(settings(), [capability()], t);
    expect(fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string"))).toBe(true);
  });
});
