import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { cliParameterDefinitions } from "../services/cli-parameter-registry";
import { supportedLocales } from "../i18n/supported-locales";
import { managedCliAgentIds } from "../types/agent";
import type { CliParameterDiagnosticCode, CliParameterErrorCode, CliParameterRemediation } from "../types/cli-parameter-profile";

// Every registry key must resolve in every registered locale, not just the two a developer happens
// to read. A missing key renders as the key itself, which is how a settings page ends up showing
// `cliParameters.codex-cli.localProvider.label` to a user.

const locales = supportedLocales.map((definition) => ({
  locale: definition.id,
  strings: JSON.parse(
    readFileSync(`src/i18n/locales/${definition.id}.json`, "utf8"),
  ) as Record<string, string>,
}));

const diagnosticCodes: readonly CliParameterDiagnosticCode[] = [
  "LEGACY_SELECTION_MIGRATED",
  "LEGACY_SELECTION_QUARANTINED",
  "UNSUPPORTED_BY_ACTIVE_VERSION",
  "UNSUPPORTED_PLATFORM",
  "UNSUPPORTED_VALUE",
  "VERSION_UNKNOWN",
  "CLI_NOT_INSTALLED",
  "ACTIVE_INSTALLATION_CONFLICT",
  "DEPENDENCY_NOT_SATISFIED",
  "CONFLICTING_SELECTION",
  "MODEL_DEPENDENT_VALUE",
  "MISSING_DIRECTORY",
  "CATALOG_REVIEW_REQUIRED",
  "REVISION_CONFLICT",
  "CATALOG_VERSION_CONFLICT",
];

const errorCodes: readonly CliParameterErrorCode[] = [
  "CLI_PARAMETER_UNKNOWN_AGENT",
  "CLI_PARAMETER_UNKNOWN_PARAMETER",
  "CLI_PARAMETER_INVALID_VALUE",
  "CLI_PARAMETER_DEPENDENCY_UNSATISFIED",
  "CLI_PARAMETER_CONFLICT",
  "CLI_PARAMETER_UNSUPPORTED_VERSION",
  "CLI_PARAMETER_REVISION_CONFLICT",
  "CLI_PARAMETER_CATALOG_MISMATCH",
  "CLI_PARAMETER_CATALOG_INVALID",
  "CLI_PARAMETER_REPOSITORY_FAILURE",
];

const remediations: readonly CliParameterRemediation[] = [
  "repair-selection",
  "adjust-dependency",
  "reselect-directory",
  "reload-profile",
  "open-cli-management",
];

function registryKeys(): string[] {
  const keys = new Set<string>();
  for (const agentId of managedCliAgentIds) {
    for (const definition of cliParameterDefinitions(agentId)) {
      keys.add(definition.labelKey);
      keys.add(definition.descriptionKey);
      for (const option of definition.options) {
        keys.add(option.labelKey);
        keys.add(option.descriptionKey);
      }
    }
  }
  return [...keys].sort();
}

function pageKeys(): string[] {
  return [
    ...diagnosticCodes.map((code) => `cliParameters.diagnostics.${code}`),
    ...errorCodes.map((code) => `cliParameters.error.${code}`),
    ...remediations.map((remediation) => `cliParameters.remediation.${remediation}`),
    ...["model", "experience", "context", "runtime", "diagnostics"].map(
      (category) => `cliParameters.category.${category}`,
    ),
    ...["stable", "preview", "experimental", "deprecated"].map(
      (maturity) => `cliParameters.maturity.${maturity}`,
    ),
    ...["all", "modified", "warnings", "unsupported", "advanced"].map(
      (filter) => `cliParameters.filters.${filter}`,
    ),
    ...[
      "supported",
      "notInstalled",
      "unknownVersion",
      "unsupportedVersion",
      "unsupportedPlatform",
    ].map((state) => `cliParameters.support.${state}`),
    "cliParameters.values.inherit.label",
    "cliParameters.values.inherit.description",
    "cliParameters.actions.restoreInherited",
    "cliParameters.actions.discardDraft",
    "cliParameters.actions.copyArgv",
    "cliParameters.actions.copied",
    "cliParameters.actions.reload",
    "cliParameters.preview.globalSegment",
    "cliParameters.preview.invocationSegment",
    "cliParameters.preview.refreshing",
    "cliParameters.preview.stale",
    "cliParameters.preview.tokenList",
    "cliParameters.conflict.title",
    "cliParameters.conflict.body",
    "cliParameters.guard.title",
    "cliParameters.guard.body",
    "cliParameters.dependency.requires",
    "cliParameters.dependency.conflictsWith",
    "cliParameters.lifecycle.version",
    "cliParameters.lifecycle.unknownVersion",
    "cliParameters.lifecycle.notInstalled",
    "cliParameters.lifecycle.notRunnable",
    "cliParameters.lifecycle.conflict",
    "cliParameters.lifecycle.manage",
    "cliParameters.badge.dirty",
    "cliParameters.badge.warnings",
    "cliParameters.badge.errors",
    "cliParameters.scopeSelector.label",
    "cliParameters.scope.chatShort",
    "cliParameters.scope.interactiveShort",
    "cliParameters.search.label",
    "cliParameters.search.placeholder",
    "cliParameters.filters.label",
    "cliParameters.list.placeholder",
    "cliParameters.onepieceLink",
    "cliParameters.source",
    "cliParameters.empty.filtered",
  ];
}

describe("CLI parameter localization", () => {
  it("resolves every registry key in every registered locale", () => {
    const keys = registryKeys();
    expect(keys.length).toBeGreaterThan(100);
    for (const { locale, strings } of locales) {
      const missing = keys.filter((key) => typeof strings[key] !== "string" || strings[key] === "");
      expect({ locale, missing }).toEqual({ locale, missing: [] });
    }
  });

  it("resolves every page key in every registered locale", () => {
    const keys = pageKeys();
    for (const { locale, strings } of locales) {
      const missing = keys.filter((key) => typeof strings[key] !== "string" || strings[key] === "");
      expect({ locale, missing }).toEqual({ locale, missing: [] });
    }
  });

  it("never invents a flag that the registry does not have", () => {
    // Quoting a flag in prose is correct and often necessary; translating one is not. Rather than
    // forbid the mention, this checks that every flag-shaped token in every locale is a flag the
    // registry actually emits, which is what a mistranslation would violate.
    const known = new Set<string>();
    for (const agentId of managedCliAgentIds) {
      for (const definition of cliParameterDefinitions(agentId)) {
        const renderer = definition.renderer;
        if (renderer.kind === "positive-negative-flag") {
          known.add(renderer.positiveFlag);
          known.add(renderer.negativeFlag);
        } else {
          known.add(renderer.flag);
        }
      }
    }
    expect(known.size).toBeGreaterThan(20);

    for (const { locale, strings } of locales) {
      const offenders = Object.entries(strings)
        .filter(([key]) => key.startsWith("cliParameters."))
        .flatMap(([key, value]) =>
          [...value.matchAll(/--[a-z][a-z0-9-]*/g)]
            .map((match) => match[0])
            .filter((flag) => !known.has(flag))
            .map((flag) => `${key}: ${flag}`),
        );
      expect({ locale, offenders }).toEqual({ locale, offenders: [] });
    }
  });
});
