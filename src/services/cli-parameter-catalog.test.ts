import { describe, expect, it } from "vitest";
import { managedCliAgentIds } from "../types/agent";
import type { CliParameterDefinition } from "../types/agent";
import en from "../i18n/locales/en.json";
import zhCN from "../i18n/locales/zh-CN.json";
import {
  buildCliParameterPreview,
  buildCliParameterPreviewFromDefinitions,
  cliParameterCatalog,
  createCliParameterProfile,
  normalizeCliParameterSelections,
} from "./cli-parameter-catalog";

const reservedFlags = new Set(["--output-format", "--resume", "--session", "--json", "--format", "--prompt"]);
const expectedParameterIds = {
  "claude-code": ["model", "effort", "permissionMode", "chrome"],
  "codex-cli": ["model", "reasoningEffort", "sandbox", "approvalPolicy", "ephemeral", "strictConfig"],
  "gemini-cli": ["model", "approvalMode", "sandbox"],
  opencode: ["agent", "thinking", "autoApprove"],
} as const;

describe("CLI parameter catalog", () => {
  it("defines safe typed controls for all managed CLIs", () => {
    expect(Object.keys(cliParameterCatalog)).toEqual(managedCliAgentIds);
    for (const agentId of managedCliAgentIds) {
      const definitions = cliParameterCatalog[agentId];
      expect(definitions.map((definition) => definition.id)).toEqual(expectedParameterIds[agentId]);
      expect(definitions.some((entry) => entry.control === "enum")).toBe(true);
      expect(definitions.some((entry) => entry.control === "boolean")).toBe(true);
      expect(new Set(definitions.map((entry) => entry.id)).size).toBe(definitions.length);
      for (const definition of definitions) {
        expect(definition.agentId).toBe(agentId);
        expect(definition.launchScopes.length).toBeGreaterThan(0);
        expect(["normal", "warning"]).toContain(definition.risk);
        expect(reservedFlags.has(definition.flag)).toBe(false);
        if (definition.control === "enum") {
          expect(definition.options.some((entry) => entry.value === definition.defaultValue)).toBe(true);
        }
      }
    }
  });

  it("normalizes defaults and rejects unknown or invalid values atomically", () => {
    const baseline = createCliParameterProfile("codex-cli");
    expect(baseline.selections.sandbox).toBe("default");
    expect(() => normalizeCliParameterSelections("codex-cli", { unknown: true })).toThrow("Unknown CLI parameter");
    expect(() => normalizeCliParameterSelections("codex-cli", { sandbox: "danger-full-access" })).toThrow("Invalid value");
  });

  it("builds tokenized previews without runtime-owned or secret content", () => {
    const preview = buildCliParameterPreview("claude-code", {
      model: "sonnet",
      effort: "high",
      permissionMode: "plan",
      chrome: false,
    });
    expect(preview).toEqual(["--model", "sonnet", "--effort", "high", "--permission-mode", "plan"]);
    expect(preview.join(" ")).not.toMatch(/prompt|resume|session|token|secret/i);
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
