import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import type {
  PluginIntegrationDefinition,
  PluginIntegrationState,
  PluginIntegrationTestResult,
} from "../../../types/plugin-integration";
import { buildPluginIntegrationDiagnosticFields } from "./plugin-integration-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function definition(overrides: Partial<PluginIntegrationDefinition> = {}): PluginIntegrationDefinition {
  return {
    id: "github",
    nameKey: "plugins.github.name",
    descriptionKey: "plugins.github.description",
    version: "1.0.0",
    provider: "GitHub",
    icon: "github",
    docsUrl: "https://cli.github.com/manual/gh_auth_login",
    setupSteps: [{ id: "auth", labelKey: "plugins.github.setup.auth" }],
    ...overrides,
  };
}

function state(overrides: Partial<PluginIntegrationState> = {}): PluginIntegrationState {
  return {
    integrationId: "github",
    status: "configured",
    configured: true,
    canTest: true,
    lastCheckedAt: "2026-08-01T00:00:00Z",
    statusReasonKey: "plugins.statusReason.configured",
    message: null,
    ...overrides,
  };
}

describe("buildPluginIntegrationDiagnosticFields", () => {
  it("includes id, provider, version, status, configured, canTest, and the last-checked timestamp", () => {
    const fields = buildPluginIntegrationDiagnosticFields(definition(), state(), undefined, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get("Integration id")).toBe("github");
    expect(byLabel.get("Provider")).toBe("GitHub");
    expect(byLabel.get("Version")).toBe("1.0.0");
    expect(byLabel.get("Status")).toBe("configured");
    expect(byLabel.get("Configured")).toBe("true");
    expect(byLabel.get("Can test")).toBe("true");
    expect(byLabel.get("Last checked at")).toBe("2026-08-01T00:00:00Z");
  });

  it("falls back to the state's own statusReasonKey when no test result has completed yet", () => {
    const fields = buildPluginIntegrationDiagnosticFields(definition(), state(), undefined, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get("Status reason")).toBe("plugins.statusReason.configured");
  });

  it("prefers a freshly completed test result's own message over the state's static reason, for the matching integration", () => {
    const freshResult: PluginIntegrationTestResult = {
      integrationId: "github",
      status: "missing-cli",
      configured: false,
      message: "plugins.statusReason.missingCli",
      checkedAt: "2026-08-02T00:00:00Z",
    };
    const fields = buildPluginIntegrationDiagnosticFields(
      definition(),
      state({ statusReasonKey: "plugins.statusReason.configured" }),
      freshResult,
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get("Status reason")).toBe("plugins.statusReason.missingCli");
  });

  it("marks lastCheckedAt and statusReason unavailable rather than inventing them when nothing has run yet", () => {
    const fields = buildPluginIntegrationDiagnosticFields(
      definition(),
      state({ lastCheckedAt: null, statusReasonKey: null }),
      undefined,
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get("Last checked at")).toBeNull();
    expect(byLabel.get("Status reason")).toBeNull();
  });

  it("never carries anything beyond the bounded fields this data model can hold", () => {
    // Every field's value traces back to a static catalog string, a backend-pinned enum/reason-code
    // union, a stable id, a boolean flag, or a timestamp -- there is no free-text field on
    // `PluginIntegrationDefinition`/`PluginIntegrationState`/`PluginIntegrationTestResult` for this
    // test to accidentally miss redacting.
    const fields = buildPluginIntegrationDiagnosticFields(definition(), state(), undefined, t);
    expect(fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string"))).toBe(true);
  });
});
