import { z } from "zod";
import generatedCatalog from "../generated/cli-parameter-catalog.json";
import { managedCliAgentIds } from "../types/agent";
import type {
  CliParameterCatalog,
  CliParameterDefinition,
  CliParameterSelection,
  CliParameterSelections,
} from "../types/cli-parameter";
import type { ManagedCliAgentId } from "../types/agent";

// The generated artifact is parsed, not asserted. A bare `as CliParameterCatalog` would let a
// generator regression reach the Web adapter as a runtime shape mismatch instead of a load-time
// failure, and the adapter is the only place a wrong catalog cannot be caught by the native tests.

const agentIdSchema = z.enum(managedCliAgentIds as unknown as [string, ...string[]]);
const slotSchema = z.enum(["global", "invocation"]);
const scopeSchema = z.enum(["interactive", "chat"]);
const platformSchema = z.enum(["windows", "macos", "linux"]);

const compatibilitySchema = z.object({
  minVersion: z.string().optional(),
  maxVersion: z.string().optional(),
  platforms: z.array(platformSchema),
});

const selectionSchema: z.ZodType<CliParameterSelection> = z.union([
  z.object({ state: z.literal("inherit") }),
  z.object({
    state: z.literal("value"),
    value: z.union([z.string(), z.boolean(), z.array(z.string())]),
  }),
]);

const optionSchema = z.object({
  value: z.string(),
  labelKey: z.string(),
  descriptionKey: z.string(),
  compatibility: compatibilitySchema.optional(),
});

const rendererSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("presence-flag"), flag: z.string(), slot: slotSchema }),
  z.object({
    kind: z.literal("positive-negative-flag"),
    positiveFlag: z.string(),
    negativeFlag: z.string(),
    slot: slotSchema,
  }),
  z.object({ kind: z.literal("flag-value"), flag: z.string(), slot: slotSchema }),
  z.object({ kind: z.literal("repeat-flag-value"), flag: z.string(), slot: slotSchema }),
  z.object({
    kind: z.literal("joined-list"),
    flag: z.string(),
    separator: z.string(),
    slot: slotSchema,
  }),
  z.object({
    kind: z.literal("config-key-value"),
    flag: z.string(),
    key: z.string(),
    encoding: z.enum(["toml-string", "toml-boolean"]),
    slot: slotSchema,
  }),
]);

const constraintsSchema = z.object({
  maxLength: z.number().optional(),
  pattern: z.string().optional(),
  maxItems: z.number().optional(),
  itemMaxLength: z.number().optional(),
  itemPattern: z.string().optional(),
  dedupe: z.boolean(),
  exclusiveValues: z.array(z.string()).default([]),
  ordering: z.enum(["catalog", "user"]).optional(),
});

const conditionSchema = z.object({
  parameterId: z.string(),
  operator: z.enum(["equals", "not-inherit", "contains"]),
  value: z.union([z.string(), z.boolean()]).optional(),
});

const dependenciesSchema = z.object({
  requiresAll: z.array(conditionSchema).default([]),
  conflictsWith: z.array(z.string()).default([]),
});

const definitionSchema = z.object({
  id: z.string(),
  agentId: agentIdSchema,
  category: z.enum(["model", "experience", "context", "runtime", "diagnostics"]),
  ownership: z.enum(["user-editable", "policy-governed", "runtime-reserved"]),
  maturity: z.enum(["stable", "preview", "experimental", "deprecated"]),
  control: z.enum([
    "enum",
    "boolean-flag",
    "tri-state",
    "multi-enum",
    "custom-text",
    "ordered-string-list",
    "path-list",
  ]),
  labelKey: z.string(),
  descriptionKey: z.string(),
  defaultSelection: selectionSchema,
  launchScopes: z.array(scopeSchema),
  risk: z.enum(["normal", "warning"]),
  advanced: z.boolean(),
  options: z.array(optionSchema).default([]),
  renderer: rendererSchema,
  constraints: constraintsSchema,
  compatibility: compatibilitySchema,
  dependencies: dependenciesSchema,
  diagnostics: z.array(z.string()).default([]),
});

const catalogSchema = z.object({
  catalogVersion: z.string(),
  selectionSchemaVersion: z.number(),
  agents: z.array(z.object({ agentId: agentIdSchema, parameters: z.array(definitionSchema) })),
});

function loadCatalog(): CliParameterCatalog {
  const parsed = catalogSchema.safeParse(generatedCatalog);
  if (!parsed.success) {
    throw new Error(
      `src/generated/cli-parameter-catalog.json does not match the CLI parameter contract: ${parsed.error.message}`,
    );
  }
  return parsed.data as CliParameterCatalog;
}

const catalog = loadCatalog();

export const cliParameterCatalogVersion = catalog.catalogVersion;
export const cliParameterSelectionSchemaVersion = catalog.selectionSchemaVersion;

/** Every definition for an agent, registry order preserved. */
export function cliParameterDefinitions(agentId: ManagedCliAgentId): CliParameterDefinition[] {
  return catalog.agents.find((agent) => agent.agentId === agentId)?.parameters ?? [];
}

/** Only what the settings page may edit. Policy-governed and runtime-reserved entries stay with
 * their owning path and must never appear as a user control. */
export function editableCliParameterDefinitions(
  agentId: ManagedCliAgentId,
): CliParameterDefinition[] {
  return cliParameterDefinitions(agentId).filter(
    (definition) => definition.ownership === "user-editable",
  );
}

export function defaultCliParameterSelections(agentId: ManagedCliAgentId): CliParameterSelections {
  return Object.fromEntries(
    editableCliParameterDefinitions(agentId).map((definition) => [
      definition.id,
      definition.defaultSelection,
    ]),
  );
}
