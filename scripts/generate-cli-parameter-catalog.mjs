#!/usr/bin/env node
// Generates the frontend CLI-parameter registry contract from the canonical native registry.
//
// The canonical registry is Rust-owned. This script only projects it: it applies the same field
// defaults Serde applies, drops the native-only audit prose, and writes a deterministic JSON
// artifact the Web/mock adapter consumes. `--check` fails when the committed artifact has drifted.

import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SOURCE = resolve(
  root,
  "src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json",
);
const TARGET = resolve(root, "src/generated/cli-parameter-catalog.json");

const MANAGED_CLI_AGENT_IDS = [
  "claude-code",
  "codex-cli",
  "gemini-cli",
  "opencode",
  "antigravity-cli",
];

const ALL_PLATFORMS = ["windows", "macos", "linux"];

const RESERVED_FLAGS = new Set([
  "-p",
  "-o",
  "-c",
  "-",
  "--prompt",
  "--print",
  "--output-format",
  "--format",
  "--json",
  "--resume",
  "--session",
  "--session-id",
  "--conversation",
  "--include-partial-messages",
  "--verbose",
  "--append-system-prompt",
]);

const FORBIDDEN_FLAG_SUBSTRINGS = [
  "dangerously",
  "api-key",
  "api_key",
  "token",
  "password",
  "secret",
  "system-prompt",
  "stdin",
];

const RENDERER_BY_CONTROL = {
  enum: ["flag-value", "config-key-value"],
  "custom-text": ["flag-value", "config-key-value"],
  "boolean-flag": ["presence-flag", "config-key-value"],
  "tri-state": ["positive-negative-flag", "config-key-value"],
  "multi-enum": ["repeat-flag-value", "joined-list"],
  "ordered-string-list": ["repeat-flag-value", "joined-list"],
  "path-list": ["repeat-flag-value", "joined-list"],
};

function fail(message) {
  throw new Error(`canonical CLI parameter registry is invalid: ${message}`);
}

function rendererFlags(renderer) {
  return renderer.kind === "positive-negative-flag"
    ? [renderer.positiveFlag, renderer.negativeFlag]
    : [renderer.flag];
}

function normalizeCompatibility(compatibility) {
  const normalized = {};
  if (compatibility?.minVersion !== undefined) normalized.minVersion = compatibility.minVersion;
  if (compatibility?.maxVersion !== undefined) normalized.maxVersion = compatibility.maxVersion;
  normalized.platforms = compatibility?.platforms ?? ALL_PLATFORMS;
  return normalized;
}

function normalizeConstraints(constraints = {}) {
  const normalized = {};
  for (const key of ["maxLength", "pattern", "maxItems", "itemMaxLength", "itemPattern"]) {
    if (constraints[key] !== undefined) normalized[key] = constraints[key];
  }
  normalized.dedupe = constraints.dedupe ?? false;
  if (constraints.exclusiveValues?.length) normalized.exclusiveValues = constraints.exclusiveValues;
  if (constraints.ordering !== undefined) normalized.ordering = constraints.ordering;
  return normalized;
}

function normalizeDependencies(dependencies = {}) {
  const normalized = {};
  if (dependencies.requiresAll?.length) {
    normalized.requiresAll = dependencies.requiresAll.map((condition) => {
      const entry = { parameterId: condition.parameterId, operator: condition.operator };
      if (condition.value !== undefined) entry.value = condition.value;
      return entry;
    });
  }
  if (dependencies.conflictsWith?.length) normalized.conflictsWith = dependencies.conflictsWith;
  return normalized;
}

function normalizeOption(option) {
  const normalized = {
    value: option.value,
    labelKey: option.labelKey,
    descriptionKey: option.descriptionKey,
  };
  if (option.compatibility) normalized.compatibility = normalizeCompatibility(option.compatibility);
  return normalized;
}

function normalizeParameter(agentId, parameter) {
  const normalized = {
    id: parameter.id,
    agentId,
    category: parameter.category,
    ownership: parameter.ownership ?? "user-editable",
    maturity: parameter.maturity ?? "stable",
    control: parameter.control,
    labelKey: parameter.labelKey,
    descriptionKey: parameter.descriptionKey,
    defaultSelection: parameter.defaultSelection ?? { state: "inherit" },
    launchScopes: parameter.launchScopes,
    risk: parameter.risk ?? "normal",
    advanced: parameter.advanced ?? false,
    options: (parameter.options ?? []).map(normalizeOption),
    renderer: parameter.renderer,
    constraints: normalizeConstraints(parameter.constraints),
    compatibility: normalizeCompatibility(parameter.compatibility),
    dependencies: normalizeDependencies(parameter.dependencies),
  };
  if (parameter.diagnostics?.length) normalized.diagnostics = parameter.diagnostics;
  return normalized;
}

function validate(catalog) {
  if (!/^\d+(\.\d+)*$/.test(catalog.catalogVersion ?? "")) fail("catalogVersion is unparseable");
  if (!(catalog.selectionSchemaVersion > 0)) fail("selectionSchemaVersion must be positive");
  const declared = catalog.agents.map((agent) => agent.agentId);
  if (declared.join(",") !== MANAGED_CLI_AGENT_IDS.join(",")) {
    fail(`agent ids must be exactly ${MANAGED_CLI_AGENT_IDS.join(", ")} in order`);
  }
  for (const agent of catalog.agents) {
    const ids = new Set();
    const flags = new Set();
    for (const parameter of agent.parameters) {
      if (ids.has(parameter.id)) fail(`${agent.agentId} repeats the parameter id ${parameter.id}`);
      ids.add(parameter.id);
      if (!parameter.launchScopes?.length) fail(`${parameter.id} declares no launch scope`);
      if (!RENDERER_BY_CONTROL[parameter.control]?.includes(parameter.renderer.kind)) {
        fail(`${parameter.id} pairs ${parameter.control} with ${parameter.renderer.kind}`);
      }
      if (
        parameter.renderer.kind === "positive-negative-flag" &&
        parameter.renderer.positiveFlag === parameter.renderer.negativeFlag
      ) {
        fail(`${parameter.id} uses the same flag for both tri-state directions`);
      }
      if (
        ["enum", "multi-enum"].includes(parameter.control) &&
        !(parameter.options ?? []).length
      ) {
        fail(`${parameter.id} declares no allowed values`);
      }
      if (!parameter.labelKey || !parameter.descriptionKey) {
        fail(`${parameter.id} is missing a localization key`);
      }
      if (!parameter.audit?.sourceUrl?.startsWith("https://")) {
        fail(`${parameter.id} has a non-https audit source`);
      }
      for (const flag of rendererFlags(parameter.renderer)) {
        if (RESERVED_FLAGS.has(flag)) fail(`${parameter.id} maps to the reserved flag ${flag}`);
        const lowered = flag.toLowerCase();
        if (FORBIDDEN_FLAG_SUBSTRINGS.some((entry) => lowered.includes(entry))) {
          fail(`${parameter.id} maps to the forbidden flag ${flag}`);
        }
        if (flags.has(flag)) fail(`${agent.agentId} repeats the flag ${flag}`);
        flags.add(flag);
      }
    }
    for (const parameter of agent.parameters) {
      const references = [
        ...(parameter.dependencies?.requiresAll ?? []).map((entry) => entry.parameterId),
        ...(parameter.dependencies?.conflictsWith ?? []),
      ];
      for (const reference of references) {
        if (!ids.has(reference)) fail(`${parameter.id} references unknown ${reference}`);
      }
    }
  }
}

function render(catalog) {
  validate(catalog);
  const projected = {
    // JSON carries no comments, so the generated marker is a reserved key instead of a header.
    $generated:
      "Generated by scripts/generate-cli-parameter-catalog.mjs from " +
      "src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json. " +
      "Do not edit by hand; run `npm run contracts:generate`.",
    catalogVersion: catalog.catalogVersion,
    selectionSchemaVersion: catalog.selectionSchemaVersion,
    agents: catalog.agents.map((agent) => ({
      agentId: agent.agentId,
      parameters: agent.parameters.map((parameter) => normalizeParameter(agent.agentId, parameter)),
    })),
  };
  return `${JSON.stringify(projected, null, 2)}\n`;
}

function main() {
  const check = process.argv.includes("--check");
  const output = render(JSON.parse(readFileSync(SOURCE, "utf8")));
  if (check) {
    const current = existsSync(TARGET) ? readFileSync(TARGET, "utf8") : "";
    if (current !== output) {
      console.error(
        "src/generated/cli-parameter-catalog.json is stale.\n" +
          "Run `npm run contracts:generate` and commit the result. Never hand-edit it.",
      );
      process.exit(1);
    }
    console.log("cli-parameter catalog contract is up to date");
    return;
  }
  mkdirSync(dirname(TARGET), { recursive: true });
  writeFileSync(TARGET, output, "utf8");
  console.log(`wrote ${TARGET}`);
}

main();
