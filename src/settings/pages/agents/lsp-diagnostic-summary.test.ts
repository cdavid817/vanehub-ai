import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import { formatDiagnosticSummary } from "../../../ui/diagnostics/diagnostic-field";
import type {
  LspConfiguration,
  LspLanguageConfiguration,
  LspLanguageDescriptor,
  LspServerDiscovery,
  LspServerStatus,
  LspWorkspaceTrust,
} from "../../../types/lsp";
import { buildLspDiagnosticFields } from "./lsp-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

/** Mirrors the source file's own `languageField`/`instanceField` composition, so assertions track
 *  real keys rather than hardcoded translated text. */
function languageLabel(language: string, fieldKey: string): string {
  return t("lspSettings.diagnostics.field.languagePrefixed", {
    language: t(`lspSettings.language.${language}`),
    field: t(fieldKey),
  });
}

function instanceLabel(instance: string, fieldKey: string): string {
  return t("lspSettings.diagnostics.field.instancePrefixed", { instance, field: t(fieldKey) });
}

function descriptor(overrides: Partial<LspLanguageDescriptor> = {}): LspLanguageDescriptor {
  return {
    language: "rust",
    server: "rust_analyzer",
    supportedOnHost: true,
    defaultStartupArguments: [],
    overrideTarget: "executable_file",
    prerequisite: null,
    distribution: null,
    installed: false,
    ...overrides,
  };
}

function languageConfig(overrides: Partial<LspLanguageConfiguration> = {}): LspLanguageConfiguration {
  return {
    language: "rust",
    enabled: true,
    executableOverride: null,
    startupArguments: null,
    initializationOptions: {},
    ...overrides,
  };
}

function discoveryEntry(overrides: Partial<LspServerDiscovery> = {}): LspServerDiscovery {
  return {
    language: "rust",
    server: "rust_analyzer",
    availability: "available",
    executablePath: "/mock/lsp/rust-analyzer",
    arguments: [],
    reasonCode: null,
    ...overrides,
  };
}

function trustRecord(overrides: Partial<LspWorkspaceTrust> = {}): LspWorkspaceTrust {
  return { canonicalRoot: "/home/user/project", trusted: true, revision: 1, ...overrides };
}

function statusEntry(overrides: Partial<LspServerStatus> = {}): LspServerStatus {
  return {
    language: "rust",
    server: "rust_analyzer",
    relativeProjectRoot: ".",
    state: "ready",
    restartCount: 0,
    lastResponseAt: "2026-01-01T00:00:00Z",
    diagnosticCount: 3,
    reasonCode: null,
    negotiatedCapabilities: { positionEncoding: "utf16", documentSync: "incremental", methods: [] },
    ...overrides,
  };
}

function configuration(overrides: Partial<LspConfiguration> = {}): LspConfiguration {
  return { enabled: true, languages: [languageConfig()], descriptors: [descriptor()], ...overrides };
}

describe("buildLspDiagnosticFields", () => {
  it("includes the master enabled flag and a configured language's core fields as raw values", () => {
    const fields = buildLspDiagnosticFields(configuration(), [discoveryEntry()], [], [], t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(t("lspSettings.configuration.master"))).toBe("true");
    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.enabled"))).toBe("true");
    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.supportedOnHost"))).toBe("true");
    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.overrideTarget"))).toBe("executable_file");
    expect(byLabel.get(languageLabel("rust", "lspSettings.discovery.override"))).toBeNull();
    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.discoveryAvailability"))).toBe("available");
    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.discoveryExecutablePath"))).toBe("/mock/lsp/rust-analyzer");
  });

  it("disambiguates the same field name across two configured languages with a language-prefixed label", () => {
    const fields = buildLspDiagnosticFields(
      configuration({
        languages: [languageConfig({ language: "rust", enabled: true }), languageConfig({ language: "python", enabled: false })],
        descriptors: [descriptor({ language: "rust" }), descriptor({ language: "python", server: "pyright" })],
      }),
      [],
      [],
      [],
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.enabled"))).toBe("true");
    expect(byLabel.get(languageLabel("python", "lspSettings.diagnostics.field.enabled"))).toBe("false");
  });

  it("labels an install-directory language's override with its own directory label, not the generic executable one", () => {
    const fields = buildLspDiagnosticFields(
      configuration({
        languages: [languageConfig({ language: "java", executableOverride: "/opt/jdtls" })],
        descriptors: [descriptor({ language: "java", server: "jdtls", overrideTarget: "install_directory" })],
      }),
      [],
      [],
      [],
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(languageLabel("java", "lspSettings.discovery.installDirectory"))).toBe("/opt/jdtls");
  });

  it("disambiguates two simultaneous instances of the same language and server against different project roots", () => {
    const fields = buildLspDiagnosticFields(
      configuration(),
      [],
      [],
      [statusEntry({ relativeProjectRoot: ".", restartCount: 0 }), statusEntry({ relativeProjectRoot: "../other-project", restartCount: 2 })],
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    const languageName = t("lspSettings.language.rust");
    const instanceA = `${languageName} · rust_analyzer · .`;
    const instanceB = `${languageName} · rust_analyzer · ../other-project`;

    expect(byLabel.get(instanceLabel(instanceA, "lspSettings.runtime.restartCount"))).toBe("0");
    expect(byLabel.get(instanceLabel(instanceB, "lspSettings.runtime.restartCount"))).toBe("2");
  });

  it("keeps every LspSafeReasonCode as its raw untranslated wire value, not a localized message", () => {
    const fields = buildLspDiagnosticFields(
      configuration(),
      [discoveryEntry({ availability: "unavailable", executablePath: null, reasonCode: "executable_not_found" })],
      [],
      [statusEntry({ state: "failed", reasonCode: "spawn_failed" })],
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    const instance = `${t("lspSettings.language.rust")} · rust_analyzer · .`;

    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.discoveryReasonCode"))).toBe("executable_not_found");
    expect(byLabel.get(instanceLabel(instance, "lspSettings.diagnostics.field.statusReasonCode"))).toBe("spawn_failed");
    expect(byLabel.get(instanceLabel(instance, "lspSettings.diagnostics.field.state"))).toBe("failed");
  });

  it("only includes trusted workspace roots, never a revoked one", () => {
    const fields = buildLspDiagnosticFields(
      configuration(),
      [],
      [
        trustRecord({ canonicalRoot: "/home/user/trusted-project", trusted: true }),
        trustRecord({ canonicalRoot: "/home/user/revoked-project", trusted: false }),
      ],
      [],
      t,
    );
    const summary = formatDiagnosticSummary(fields, t("workbenchUi.diagnostics.unavailable"));

    expect(summary).toContain("/home/user/trusted-project");
    expect(summary).not.toContain("/home/user/revoked-project");
  });

  it("marks a field unavailable, and omits per-language/per-instance rows entirely, before anything has loaded", () => {
    const fields = buildLspDiagnosticFields(undefined, [], [], [], t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(t("lspSettings.configuration.master"))).toBeNull();
    expect(byLabel.get(t("lspSettings.diagnostics.field.trustedWorkspaceRoots"))).toBeNull();
    expect(fields.length).toBe(2);
  });

  it("never leaks unbounded startup-argument or initialization-option content into the formatted output", () => {
    const fields = buildLspDiagnosticFields(
      configuration({
        languages: [languageConfig({ startupArguments: ["--token=SENTINEL_ARG"], initializationOptions: { key: "SENTINEL_JSON" } })],
        descriptors: [descriptor({ defaultStartupArguments: ["SENTINEL_DEFAULT_ARG"] })],
      }),
      [discoveryEntry({ arguments: ["SENTINEL_DISCOVERY_ARG"] })],
      [],
      [],
      t,
    );
    const summary = formatDiagnosticSummary(fields, t("workbenchUi.diagnostics.unavailable"));

    expect(summary).not.toContain("SENTINEL_ARG");
    expect(summary).not.toContain("SENTINEL_JSON");
    expect(summary).not.toContain("SENTINEL_DEFAULT_ARG");
    expect(summary).not.toContain("SENTINEL_DISCOVERY_ARG");
  });

  it("reports download verification only for a language with a distribution, unavailable for one with none", () => {
    const fields = buildLspDiagnosticFields(
      configuration({
        languages: [languageConfig({ language: "rust" }), languageConfig({ language: "java" })],
        descriptors: [
          descriptor({ language: "rust", distribution: null }),
          descriptor({ language: "java", server: "jdtls", distribution: { verified: false } }),
        ],
      }),
      [],
      [],
      [],
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(languageLabel("rust", "lspSettings.diagnostics.field.distributionVerified"))).toBeNull();
    expect(byLabel.get(languageLabel("java", "lspSettings.diagnostics.field.distributionVerified"))).toBe("false");
  });

  it("never carries anything beyond the bounded fields this page's data model can hold", () => {
    // Every field's value traces back to a raw enum/reason-code union, a stable path already shown
    // on screen, a version-like backend string, a plain number/boolean rendered as text, or a raw
    // timestamp -- there is no credential anywhere on this page for this test to accidentally miss
    // redacting, the same structural guarantee `cli-diagnostic-summary.test.ts` proves for CLI.
    const fields = buildLspDiagnosticFields(
      configuration({
        languages: [languageConfig()],
        descriptors: [descriptor({ prerequisite: "Java 17 or newer", distribution: { verified: true }, installed: true })],
      }),
      [discoveryEntry()],
      [trustRecord()],
      [statusEntry()],
      t,
    );
    expect(fields.length).toBeGreaterThan(15);
    expect(
      fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string")),
    ).toBe(true);
  });
});
