import {
  managedCliAgentIds,
  type CliParameterDefinition,
  type CliParameterProfile,
  type CliParameterSelections,
  type CliParameterValue,
  type ManagedCliAgentId,
} from "../types/agent";

const defaultOption = (prefix: string) => ({
  value: "default",
  labelKey: "cliParameters.values.default.label",
  descriptionKey: `${prefix}.values.default.description`,
});

const option = (prefix: string, value: string) => ({
  value,
  labelKey: ["low", "medium", "high", "xhigh", "max"].includes(value)
    ? `cliParameters.common.values.${value}.label`
    : `${prefix}.values.${value}.label`,
  descriptionKey: ["low", "medium", "high", "xhigh", "max"].includes(value)
    ? `cliParameters.common.values.${value}.description`
    : `${prefix}.values.${value}.description`,
});

function enumDefinition(
  agentId: ManagedCliAgentId,
  id: string,
  flag: string,
  values: string[],
  scopes: Array<"interactive" | "chat"> = ["interactive", "chat"],
  defaultValue = "default",
  risk: "normal" | "warning" = "normal",
): CliParameterDefinition {
  const prefix = `cliParameters.${agentId}.${id}`;
  return {
    id,
    agentId,
    flag,
    control: "enum",
    labelKey: `${prefix}.label`,
    descriptionKey: `${prefix}.description`,
    options: values.map((value) => (value === "default" ? defaultOption(prefix) : option(prefix, value))),
    defaultValue,
    launchScopes: scopes,
    risk,
  };
}

function booleanDefinition(
  agentId: ManagedCliAgentId,
  id: string,
  flag: string,
  scopes: Array<"interactive" | "chat">,
  risk: "normal" | "warning" = "normal",
): CliParameterDefinition {
  const prefix = `cliParameters.${agentId}.${id}`;
  return {
    id,
    agentId,
    flag,
    control: "boolean",
    labelKey: `${prefix}.label`,
    descriptionKey: `${prefix}.description`,
    options: [],
    defaultValue: false,
    launchScopes: scopes,
    risk,
  };
}

export const cliParameterCatalog: Record<ManagedCliAgentId, CliParameterDefinition[]> = {
  "claude-code": [
    enumDefinition("claude-code", "model", "--model", ["default", "sonnet", "opus", "haiku"]),
    enumDefinition("claude-code", "effort", "--effort", ["default", "low", "medium", "high", "xhigh", "max"]),
    enumDefinition("claude-code", "permissionMode", "--permission-mode", ["default", "plan", "acceptEdits", "auto", "dontAsk"]),
    booleanDefinition("claude-code", "chrome", "--chrome", ["interactive"]),
  ],
  "codex-cli": [
    enumDefinition("codex-cli", "model", "--model", ["default", "gpt-5.5", "gpt-5.4", "gpt-5.2-codex", "gpt-5.1-codex-max"]),
    enumDefinition("codex-cli", "reasoningEffort", "--config", ["default", "low", "medium", "high", "xhigh", "max"]),
    enumDefinition("codex-cli", "sandbox", "--sandbox", ["default", "read-only", "workspace-write"]),
    enumDefinition("codex-cli", "approvalPolicy", "--ask-for-approval", ["default", "untrusted", "on-request", "never"]),
    booleanDefinition("codex-cli", "ephemeral", "--ephemeral", ["chat"]),
    booleanDefinition("codex-cli", "strictConfig", "--strict-config", ["interactive", "chat"]),
  ],
  "gemini-cli": [
    enumDefinition("gemini-cli", "model", "--model", ["default", "gemini-2.5-pro", "gemini-2.5-flash"]),
    enumDefinition("gemini-cli", "approvalMode", "--approval-mode", ["default", "auto_edit", "plan", "yolo"], ["interactive", "chat"], "yolo", "warning"),
    booleanDefinition("gemini-cli", "sandbox", "--sandbox", ["interactive", "chat"]),
  ],
  opencode: [
    enumDefinition("opencode", "agent", "--agent", ["default", "build", "plan"]),
    booleanDefinition("opencode", "thinking", "--thinking", ["chat"]),
    booleanDefinition("opencode", "autoApprove", "--auto", ["interactive", "chat"], "warning"),
  ],
};

export function isManagedCliAgentId(value: string): value is ManagedCliAgentId {
  return managedCliAgentIds.includes(value as ManagedCliAgentId);
}

export function defaultCliParameterSelections(agentId: ManagedCliAgentId): CliParameterSelections {
  return Object.fromEntries(cliParameterCatalog[agentId].map((definition) => [definition.id, definition.defaultValue]));
}

function isValidValue(definition: CliParameterDefinition, value: CliParameterValue): boolean {
  if (definition.control === "boolean") return typeof value === "boolean";
  if (definition.control === "enum") {
    return typeof value === "string" && definition.options.some((entry) => entry.value === value);
  }
  return Array.isArray(value) && value.every((entry) => definition.options.some((optionEntry) => optionEntry.value === entry));
}

export function normalizeCliParameterSelections(
  agentId: ManagedCliAgentId,
  input: CliParameterSelections,
): CliParameterSelections {
  const definitions = cliParameterCatalog[agentId];
  return normalizeSelectionsFromDefinitions(definitions, input);
}

function normalizeSelectionsFromDefinitions(
  definitions: CliParameterDefinition[],
  input: CliParameterSelections,
): CliParameterSelections {
  const definitionIds = new Set(definitions.map((definition) => definition.id));
  for (const id of Object.keys(input)) {
    if (!definitionIds.has(id)) throw new Error(`Unknown CLI parameter: ${id}`);
  }
  return Object.fromEntries(
    definitions.map((definition) => {
      const value = input[definition.id] ?? definition.defaultValue;
      if (!isValidValue(definition, value)) throw new Error(`Invalid value for CLI parameter: ${definition.id}`);
      const normalizedValue = definition.control === "multi-enum" && Array.isArray(value)
        ? definition.options.filter((optionEntry) => value.includes(optionEntry.value)).map((optionEntry) => optionEntry.value)
        : value;
      return [definition.id, normalizedValue];
    }),
  );
}

export function buildCliParameterPreview(
  agentId: ManagedCliAgentId,
  selections: CliParameterSelections,
  scope: "interactive" | "chat" = "chat",
): string[] {
  return buildCliParameterPreviewFromDefinitions(cliParameterCatalog[agentId], selections, scope);
}

export function buildCliParameterPreviewFromDefinitions(
  definitions: CliParameterDefinition[],
  selections: CliParameterSelections,
  scope: "interactive" | "chat" = "chat",
): string[] {
  const normalized = normalizeSelectionsFromDefinitions(definitions, selections);
  const args: string[] = [];
  for (const definition of definitions) {
    if (!definition.launchScopes.includes(scope)) continue;
    const value = normalized[definition.id];
    if (definition.control === "boolean") {
      if (value === true) args.push(definition.flag);
    } else if (definition.control === "enum") {
      if (typeof value === "string" && value !== "default") {
        const renderedValue = definition.id === "reasoningEffort" ? `model_reasoning_effort="${value}"` : value;
        args.push(definition.flag, renderedValue);
      }
    } else if (Array.isArray(value)) {
      for (const entry of value) args.push(definition.flag, entry);
    }
  }
  return args;
}

export function createCliParameterProfile(
  agentId: ManagedCliAgentId,
  selections: CliParameterSelections = defaultCliParameterSelections(agentId),
): CliParameterProfile {
  const normalized = normalizeCliParameterSelections(agentId, selections);
  return {
    agentId,
    definitions: cliParameterCatalog[agentId],
    selections: normalized,
    previewArgs: buildCliParameterPreview(agentId, normalized),
  };
}
