import { describe, expect, it } from "vitest";
import { managedCliAgentIds } from "../types/agent";
import type { CliParameterDefinition } from "../types/agent";
import en from "../i18n/locales/en.json";
import zhCN from "../i18n/locales/zh-CN.json";
import editableCatalogContract from "../contracts/fixtures/cli-parameter-editable-catalog.json";
import {
  buildCliParameterPreview,
  buildCliParameterPreviewFromDefinitions,
  cliParameterCatalog,
  createCliParameterProfile,
  normalizeCliParameterSelections,
} from "./cli-parameter-catalog";

const reservedFlags = new Set(["--output-format", "--resume", "--session", "--json", "--format", "--prompt"]);
const expectedParameterIds = {
  "claude-code": ["model", "effort", "chrome", "agent", "advisor", "disableSlashCommands", "screenReader", "bare", "safeMode"],
  "codex-cli": ["model", "reasoningEffort", "ephemeral", "strictConfig", "profile", "search", "oss", "noAltScreen"],
  "gemini-cli": ["model", "debug", "screenReader"],
  opencode: ["model", "variant", "thinking", "pure", "printLogs", "logLevel"],
  "antigravity-cli": ["model", "effort", "agent"],
} as const;

describe("CLI parameter catalog", () => {
  it("matches the shared frontend/native editable catalog contract", () => {
    const actual = Object.fromEntries(managedCliAgentIds.map((agentId) => [
      agentId,
      cliParameterCatalog[agentId].map((definition) => ({
        id: definition.id,
        flag: definition.flag,
        launchScopes: definition.launchScopes,
        options: definition.options.map((optionEntry) => optionEntry.value),
      })),
    ]));
    expect(actual).toEqual(editableCatalogContract);
  });

  it("defines safe typed controls for all managed CLIs", () => {
    expect(Object.keys(cliParameterCatalog)).toEqual(managedCliAgentIds);
    for (const agentId of managedCliAgentIds) {
      const definitions = cliParameterCatalog[agentId];
      expect(definitions.map((definition) => definition.id)).toEqual(expectedParameterIds[agentId]);
      expect(definitions.length).toBeGreaterThan(0);
      expect(new Set(definitions.map((entry) => entry.id)).size).toBe(definitions.length);
      for (const definition of definitions) {
        expect(definition.agentId).toBe(agentId);
        expect(definition.launchScopes.length).toBeGreaterThan(0);
        expect(["normal", "warning"]).toContain(definition.risk);
        expect(reservedFlags.has(definition.flag)).toBe(false);
        expect(definition.flag).not.toContain("dangerously");
        expect(definition.flag).not.toBe("--conversation");
        if (definition.control === "enum" || definition.control === "custom-text") {
          expect(definition.options.some((entry) => entry.value === definition.defaultValue)).toBe(true);
        }
      }
    }
  });

  it("normalizes defaults and rejects unknown or invalid values atomically", () => {
    const baseline = createCliParameterProfile("codex-cli");
    expect(baseline.selections.ephemeral).toBe(false);
    expect(() => normalizeCliParameterSelections("codex-cli", { unknown: true })).toThrow("Unknown CLI parameter");
    expect(() => normalizeCliParameterSelections("codex-cli", { sandbox: "read-only" })).toThrow("Unknown CLI parameter");
    expect(() => normalizeCliParameterSelections("codex-cli", { reasoningEffort: "impossible" })).toThrow("Invalid value");
  });

  it("builds tokenized previews without runtime-owned or secret content", () => {
    const preview = buildCliParameterPreview("claude-code", {
      model: "sonnet",
      effort: "high",
      chrome: false,
    });
    expect(preview).toEqual(["--model", "sonnet", "--effort", "high"]);
    expect(preview.join(" ")).not.toMatch(/prompt|resume|session|token|secret/i);
  });

  it("uses the audited Gemini aliases and scopes OpenCode variants to chat runs", () => {
    expect(cliParameterCatalog["gemini-cli"][0].options.map((entry) => entry.value))
      .toEqual(["default", "auto", "pro", "flash", "flash-lite"]);
    expect(cliParameterCatalog.opencode.find((entry) => entry.id === "model")?.launchScopes)
      .toEqual(["interactive", "chat"]);
    expect(cliParameterCatalog.opencode.find((entry) => entry.id === "variant")?.launchScopes)
      .toEqual(["chat"]);
    expect(buildCliParameterPreview("opencode", {
      model: "anthropic/claude-sonnet-4-5",
      variant: "high",
      thinking: true,
    }, "chat")).toEqual([
      "--model",
      "anthropic/claude-sonnet-4-5",
      "--variant",
      "high",
      "--thinking",
    ]);
  });

  it("names every managed CLI in each Agent-keyed copy namespace", () => {
    // Several surfaces build a translation key from an Agent id, each in its own namespace, and
    // each renders the raw key when an entry is missing. Adding an Agent with parameter copy but
    // no entry in these namespaces is invisible to the per-parameter checks below, so they are
    // enumerated here rather than discovered one bug report at a time.
    const resources: Array<Record<string, string>> = [en, zhCN];
    const namespaces = [
      (agentId: string) => `cliParameters.agents.${agentId}`,
      (agentId: string) => `layout.agentFilter.${agentId}`,
    ];
    for (const agentId of managedCliAgentIds) {
      for (const build of namespaces) {
        const key = build(agentId);
        expect(resources.every((resource) => Boolean(resource[key])), key).toBe(true);
      }
    }
  });

  it("provides bilingual copy for every parameter and value", () => {
    const resources: Array<Record<string, string>> = [en, zhCN];
    for (const definitions of Object.values(cliParameterCatalog)) {
      for (const definition of definitions) {
        for (const key of [
          definition.labelKey,
          definition.descriptionKey,
          ...definition.options.flatMap((option) => [option.labelKey, option.descriptionKey]),
        ]) {
          expect(resources.every((resource) => Boolean(resource[key])), key).toBe(true);
        }
      }
    }
  });

  it("renders repeatable enum values in catalog order without duplicates", () => {
    const definitions: CliParameterDefinition[] = [{
      id: "feature",
      agentId: "codex-cli",
      flag: "--feature",
      control: "multi-enum",
      labelKey: "feature.label",
      descriptionKey: "feature.description",
      options: [
        { value: "alpha", labelKey: "alpha.label", descriptionKey: "alpha.description" },
        { value: "beta", labelKey: "beta.label", descriptionKey: "beta.description" },
      ],
      defaultValue: [],
      launchScopes: ["chat"],
      risk: "normal",
    }];
    expect(buildCliParameterPreviewFromDefinitions(definitions, { feature: ["beta", "alpha", "beta"] }))
      .toEqual(["--feature", "alpha", "--feature", "beta"]);
  });
});
