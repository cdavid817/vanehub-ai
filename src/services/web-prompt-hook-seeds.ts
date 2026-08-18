import type { ManagedCliAgentId } from "../types/agent";
import type { PromptHook, PromptHookCategory } from "../types/prompt-hook";

export const defaultPromptHookBindings: ManagedCliAgentId[] = ["claude-code", "codex-cli", "gemini-cli", "opencode"];
export const promptHookCategories: PromptHookCategory[] = ["bootstrap", "callback", "dynamic", "law", "navigation", "routing", "static"];

function createBuiltinPromptHook(input: {
  id: string;
  name: string;
  description: string;
  category: PromptHookCategory;
  stage: PromptHook["stage"];
  order: number;
  disableable: boolean;
  templateBody: string;
  enabled?: boolean;
}): PromptHook {
  return {
    id: input.id,
    name: input.name,
    description: input.description,
    category: input.category,
    stage: input.stage,
    order: input.order,
    version: 1,
    source: "builtin",
    enabled: input.enabled ?? true,
    disableable: input.disableable,
    cliBindings: [...defaultPromptHookBindings],
    governance: {
      safetyTier: "readonly",
      transparencyTier: input.disableable ? "opt-in-view" : "visible-by-default",
      governanceTier: input.disableable ? "human-gated" : "immutable",
    },
    templateBody: input.templateBody,
    createdAt: "2026-07-18T00:00:00.000Z",
    updatedAt: "2026-07-18T00:00:00.000Z",
  };
}

export const builtinPromptHookSeeds: PromptHook[] = [
  createBuiltinPromptHook({
    id: "bootstrap-session-context",
    name: "Session Context",
    description: "Adds session and workspace context to each CLI prompt.",
    category: "bootstrap",
    stage: "session-init",
    order: 100,
    disableable: true,
    templateBody: "Session context: {{sampleInput}}",
  }),
  createBuiltinPromptHook({
    id: "law-runtime-boundary",
    name: "Runtime Boundary",
    description: "Keeps CLI behavior inside VaneHub runtime and permission boundaries.",
    category: "law",
    stage: "session-init",
    order: 200,
    disableable: false,
    templateBody: "Respect the active VaneHub runtime, permissions, and project boundaries.",
  }),
  createBuiltinPromptHook({
    id: "static-response-format",
    name: "Response Format",
    description: "Sets a concise engineering response baseline.",
    category: "static",
    stage: "session-init",
    order: 300,
    disableable: true,
    templateBody: "Use direct, actionable engineering responses with concise verification notes.",
  }),
  createBuiltinPromptHook({
    id: "dynamic-session-config",
    name: "Session Configuration",
    description: "Summarizes active session configuration for the selected CLI.",
    category: "dynamic",
    stage: "per-turn",
    order: 400,
    disableable: true,
    templateBody: "Active CLI: {{agentId}}. User request follows after the hook context.",
  }),
  createBuiltinPromptHook({
    id: "navigation-project-hints",
    name: "Project Navigation",
    description: "Encourages grounded project inspection before code changes.",
    category: "navigation",
    stage: "per-turn",
    order: 500,
    disableable: true,
    templateBody: "Inspect relevant project files and existing patterns before making changes.",
  }),
  createBuiltinPromptHook({
    id: "routing-cli-capabilities",
    name: "CLI Capability Routing",
    description: "Keeps behavior aligned with the selected CLI agent capabilities.",
    category: "routing",
    stage: "per-turn",
    order: 600,
    disableable: true,
    templateBody: "Route work through capabilities available to {{agentId}}.",
  }),
  createBuiltinPromptHook({
    id: "callback-future-channel",
    name: "Callback Channel Placeholder",
    description: "Reserved placeholder for future callback-aware workflows.",
    category: "callback",
    stage: "per-turn",
    order: 700,
    disableable: true,
    enabled: false,
    templateBody: "Callback channel support is not active in this runtime.",
  }),
];
