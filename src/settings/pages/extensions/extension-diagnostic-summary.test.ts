import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import type { ExtensionFrameworkDefinition, ExtensionFrameworkStatus } from "../../../types/extension";
import { buildExtensionDiagnosticFields } from "./extension-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function definition(overrides: Partial<ExtensionFrameworkDefinition> = {}): ExtensionFrameworkDefinition {
  return {
    id: "paddleocr",
    capabilityId: "ocr",
    nameKey: "extensions.framework.paddleocr.name",
    descriptionKey: "extensions.framework.paddleocr.description",
    defaultPort: 9875,
    requirement: {
      runtime: "Python 3.10+",
      packages: ["paddleocr", "paddlepaddle"],
      estimatedDownloadMb: 650,
      estimatedDiskMb: 1800,
      models: [{ id: "PP-OCRv5-mobile", sizeMb: 120, descriptionKey: "extensions.model.paddleocr" }],
    },
    ...overrides,
  };
}

function status(overrides: Partial<ExtensionFrameworkStatus> = {}): ExtensionFrameworkStatus {
  return {
    frameworkId: "paddleocr",
    capabilityId: "ocr",
    status: "running",
    installed: true,
    enabled: true,
    running: true,
    port: 9875,
    installPath: "/mock/extensions/paddleocr",
    installedVersion: "2.7.0",
    lastHealthCheck: "2026-01-01T00:00:00.000Z",
    lastError: null,
    lastOperationId: "op-123",
    ...overrides,
  };
}

describe("buildExtensionDiagnosticFields", () => {
  it("includes stable ids, installed version, lifecycle flags, and the reused on-page requirement labels", () => {
    const fields = buildExtensionDiagnosticFields(definition(), status(), t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(t("extensions.diagnostics.field.frameworkId"))).toBe("paddleocr");
    expect(byLabel.get(t("extensions.diagnostics.field.capabilityId"))).toBe("ocr");
    expect(byLabel.get(t("extensions.diagnostics.field.installedVersion"))).toBe("2.7.0");
    expect(byLabel.get(t("extensions.diagnostics.field.status"))).toBe("running");
    expect(byLabel.get(t("extensions.diagnostics.field.installed"))).toBe("true");
    expect(byLabel.get(t("extensions.diagnostics.field.enabled"))).toBe("true");
    expect(byLabel.get(t("extensions.diagnostics.field.running"))).toBe("true");
    expect(byLabel.get(t("extensions.diagnostics.field.installPath"))).toBe("/mock/extensions/paddleocr");
    expect(byLabel.get(t("extensions.diagnostics.field.lastHealthCheck"))).toBe("2026-01-01T00:00:00.000Z");
    expect(byLabel.get(t("extensions.diagnostics.field.lastOperationId"))).toBe("op-123");
    // Reuses the exact labels this page's own card already renders for these three fields
    // (`extension-framework-card.tsx`'s runtime/port/disk grid), not new diagnostics-only labels.
    expect(byLabel.get(t("extensions.runtime"))).toBe("Python 3.10+");
    expect(byLabel.get(t("extensions.port"))).toBe("9875");
    expect(byLabel.get(t("extensions.disk"))).toBe("1800");
  });

  it("marks version, path, timestamp, operation id, and error code unavailable rather than omitting or inventing them when the framework was never installed", () => {
    const fields = buildExtensionDiagnosticFields(
      definition(),
      status({
        status: "not-installed",
        installed: false,
        enabled: false,
        running: false,
        installPath: null,
        installedVersion: null,
        lastHealthCheck: null,
        lastError: null,
        lastOperationId: null,
      }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("extensions.diagnostics.field.installedVersion"))).toBeNull();
    expect(byLabel.get(t("extensions.diagnostics.field.installPath"))).toBeNull();
    expect(byLabel.get(t("extensions.diagnostics.field.lastHealthCheck"))).toBeNull();
    expect(byLabel.get(t("extensions.diagnostics.field.lastOperationId"))).toBeNull();
    expect(byLabel.get(t("extensions.diagnostics.field.lastError"))).toBeNull();
    expect(byLabel.get(t("extensions.diagnostics.field.status"))).toBe("not-installed");
  });

  it("carries a desktop-only lastError as its own raw reason-code key rather than translating or dropping it", () => {
    // `web-extension-client.ts` populates `lastError` with a fixed i18n key, never free text --
    // the raw key is what lets a reader look the reason up, matching CLI's own conflict/action
    // reason codes (kept raw rather than translated for pasting).
    const fields = buildExtensionDiagnosticFields(
      definition(),
      status({ lastError: "extensions.environment.desktopOnly" }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("extensions.diagnostics.field.lastError"))).toBe("extensions.environment.desktopOnly");
  });

  it("joins more than one required package rather than dropping all but one, and marks an empty package list unavailable", () => {
    const fields = buildExtensionDiagnosticFields(definition(), status(), t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("extensions.diagnostics.field.packages"))).toBe("paddleocr, paddlepaddle");

    const emptyFields = buildExtensionDiagnosticFields(
      definition({ requirement: { runtime: "Python 3.10+", packages: [], estimatedDownloadMb: 0, estimatedDiskMb: 0, models: [] } }),
      status(),
      t,
    );
    const byLabelEmpty = new Map(emptyFields.map((field) => [field.label, field.value]));
    expect(byLabelEmpty.get(t("extensions.diagnostics.field.packages"))).toBeNull();
  });

  it("never carries anything beyond the bounded fields this definition/status pair can hold", () => {
    // Every field's value traces back to a stable id, a backend-pinned enum/reason-code union, a
    // plain boolean/number, an already-used requirement string, a local path, or a timestamp --
    // there is no free-text field on ExtensionFrameworkDefinition/ExtensionFrameworkStatus for
    // this test to accidentally miss redacting.
    const fields = buildExtensionDiagnosticFields(definition(), status(), t);
    expect(
      fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string")),
    ).toBe(true);
  });
});
