import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import type { ImConnectorView } from "../../../contracts/im";
import { formatDiagnosticSummary } from "../../../ui/diagnostics/diagnostic-field";
import { buildImConnectorDiagnosticFields } from "./im-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function view(overrides: Partial<ImConnectorView> = {}): ImConnectorView {
  return {
    descriptor: { kind: "feishu", supportsQrAuthorization: false, experimental: false, maxOutboundChars: 4000 },
    config: { kind: "feishu", enabled: true, displayName: "Team bot", publicConfig: { appId: "cli_abc123" }, credentialRef: "ref-1" },
    health: { kind: "feishu", lifecycle: "connected", generation: 3, safeErrorCode: null, updatedAt: "2026-08-01T00:00:00Z" },
    hasCredentials: true,
    ...overrides,
  };
}

const SECRET_VALUE = "sk-super-secret-app-secret-do-not-leak";

describe("buildImConnectorDiagnosticFields (redaction)", () => {
  it("never includes a secret field's value even though it lives in the same publicConfig object", () => {
    // A defensive fixture: if the manifest's secret/non-secret split were ever ignored, this
    // value sitting right next to the safe appId would be the first thing a naive "dump
    // publicConfig" implementation leaked.
    const fields = buildImConnectorDiagnosticFields(
      view({ config: { kind: "feishu", enabled: true, publicConfig: { appId: "cli_abc123", appSecret: SECRET_VALUE }, credentialRef: null } }),
      t,
    );
    const summary = formatDiagnosticSummary(fields, "unavailable");
    expect(summary).not.toContain(SECRET_VALUE);
    expect(fields.some((field) => field.label === t("im.fields.appSecret"))).toBe(false);
  });

  it("includes the manifest's non-secret field for the connector kind", () => {
    const fields = buildImConnectorDiagnosticFields(view(), t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("im.fields.appId"))).toBe("cli_abc123");
  });

  it("reports hasCredentials as a boolean flag, never the credential itself", () => {
    const fields = buildImConnectorDiagnosticFields(view({ hasCredentials: true }), t);
    const summary = formatDiagnosticSummary(fields, "unavailable");
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get("Has credentials")).toBe("true");
    expect(summary).not.toContain(SECRET_VALUE);
  });

  it("treats an unexpected non-primitive publicConfig value as unavailable rather than stringifying it", () => {
    const fields = buildImConnectorDiagnosticFields(
      view({ config: { kind: "feishu", enabled: true, publicConfig: { appId: { nested: "unexpected" } }, credentialRef: null } }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("im.fields.appId"))).toBeNull();
  });

  it("handles weixin, which has no credential-field manifest entry at all, without throwing", () => {
    expect(() => buildImConnectorDiagnosticFields(
      view({
        descriptor: { kind: "weixin", supportsQrAuthorization: true, experimental: false, maxOutboundChars: 2000 },
        config: { kind: "weixin", enabled: true, publicConfig: {}, credentialRef: null },
      }),
      t,
    )).not.toThrow();
  });

  it("includes lifecycle, kind, updatedAt, and a safe error code when one is present", () => {
    const fields = buildImConnectorDiagnosticFields(
      view({ health: { kind: "feishu", lifecycle: "error", generation: 4, safeErrorCode: "im.error.authExpired", updatedAt: "2026-08-02T00:00:00Z" } }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("im.diagnostics.field.kind"))).toBe("feishu");
    expect(byLabel.get(t("im.diagnostics.field.lifecycle"))).toBe("error");
    expect(byLabel.get(t("im.diagnostics.field.updatedAt"))).toBe("2026-08-02T00:00:00Z");
    expect(byLabel.get(t("im.diagnostics.field.safeErrorCode"))).toBe("im.error.authExpired");
  });
});
