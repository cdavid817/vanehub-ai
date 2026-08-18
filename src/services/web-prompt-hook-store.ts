import { mockAgents } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";
import { readWebMockStorage, writeWebMockStorage } from "./web-mock-storage";
import { renderWebPromptHookTemplate } from "./web-prompt-hook-versions";
import { builtinPromptHookSeeds, promptHookCategories } from "./web-prompt-hook-seeds";
import type { ManagedCliAgentId } from "../types/agent";
import { managedCliAgentIds } from "../types/agent";
import type {
  PromptAssemblyPreviewInput,
  PromptHook,
  PromptHookListResult,
  PromptHookMutationInput,
  PromptHookPreview,
  PromptHookTraceSummary,
  PromptHookUpdateInput,
} from "../types/prompt-hook";

const promptHookStorageKey = "vanehub.prompt-hooks.v1";
const promptHookTraceStorageKey = "vanehub.prompt-hook-traces.v1";

// Hooks and traces carry template text and provenance only, never a credential, so browser storage
// stays inside the "Honest Web/mock behavior" prohibition on persisting plaintext secrets.
let memoryPromptHooks: Record<string, PromptHook> = {};
let memoryPromptTraces: PromptHookTraceSummary[] = [];

export function isManagedCliAgentId(value: string): value is ManagedCliAgentId {
  return managedCliAgentIds.includes(value as ManagedCliAgentId);
}

export function validatePromptHookInput(input: PromptHookMutationInput | PromptHookUpdateInput) {
  if (!/^[a-z0-9][a-z0-9-]{2,63}$/.test(input.id)) {
    throw new Error("Invalid Prompt Hook id");
  }
  if (!input.name.trim()) throw new Error("Prompt Hook name is required");
  if (!promptHookCategories.includes(input.category)) throw new Error("Unsupported Prompt Hook category");
  if (input.stage !== "session-init" && input.stage !== "per-turn") throw new Error("Unsupported Prompt Hook stage");
  if (!Number.isFinite(input.order) || input.order < 0) throw new Error("Invalid Prompt Hook order");
  if (/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(input.templateBody)) {
    throw new Error("Prompt Hook content contains unsupported control characters");
  }
  if (!input.cliBindings.every(isManagedCliAgentId)) throw new Error("Unsupported Prompt Hook CLI binding");
}

export function readStoredPromptHooks(): Record<string, PromptHook> {
  return readWebMockStorage(promptHookStorageKey, memoryPromptHooks);
}

export function writeStoredPromptHooks(value: Record<string, PromptHook>) {
  memoryPromptHooks = value;
  writeWebMockStorage(promptHookStorageKey, value);
}

export function readPromptHookTraces(): PromptHookTraceSummary[] {
  return readWebMockStorage(promptHookTraceStorageKey, memoryPromptTraces);
}

export function writePromptHookTraces(value: PromptHookTraceSummary[]) {
  memoryPromptTraces = value.slice(0, 50);
  writeWebMockStorage(promptHookTraceStorageKey, memoryPromptTraces);
}

export function listEffectivePromptHooks(): PromptHook[] {
  const stored = readStoredPromptHooks();
  const builtins = builtinPromptHookSeeds.map((hook) => stored[hook.id] ?? hook);
  const userHooks = Object.values(stored).filter((hook) => hook.source === "user");
  return [...builtins, ...userHooks].sort((left, right) => {
    if (left.stage !== right.stage) return left.stage.localeCompare(right.stage);
    if (left.category !== right.category) return left.category.localeCompare(right.category);
    return left.order - right.order || left.id.localeCompare(right.id);
  });
}

export function promptHookStats(hooks: PromptHook[]): PromptHookListResult["stats"] {
  return {
    total: hooks.length,
    enabled: hooks.filter((hook) => hook.enabled).length,
    builtin: hooks.filter((hook) => hook.source === "builtin").length,
    user: hooks.filter((hook) => hook.source === "user").length,
  };
}

export function renderPromptHookTemplate(template: string, input: { agentId: ManagedCliAgentId; sampleInput: string }) {
  const agentName = mockAgents.find((agent) => agent.id === input.agentId)?.displayName ?? input.agentId;
  return renderWebPromptHookTemplate(template, {
    agentId: input.agentId,
    agentName,
    currentTime: nowIso(),
    sampleInput: input.sampleInput,
    sessionId: "session-preview",
  });
}

function promptHookHash(content: string) {
  let hash = 5381;
  for (let index = 0; index < content.length; index += 1) {
    hash = (hash * 33) ^ content.charCodeAt(index);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

export function traceForHook(hook: PromptHook, status: PromptHookTraceSummary["status"], content: string | null, agentId: ManagedCliAgentId, reason?: string): PromptHookTraceSummary {
  return {
    id: `web-prompt-trace-${Date.now()}-${hook.id}`,
    hookId: hook.id,
    category: hook.category,
    stage: hook.stage,
    status,
    version: status === "fired" ? hook.version : undefined,
    contentHash: content ? promptHookHash(content) : undefined,
    tokenEstimate: content ? Math.ceil(content.length / 4) : undefined,
    reason,
    agentId,
    createdAt: nowIso(),
  };
}

export function assemblePromptHooks(input: PromptAssemblyPreviewInput): PromptHookPreview {
  const traces: PromptHookTraceSummary[] = [];
  const rendered: string[] = [];
  for (const hook of listEffectivePromptHooks()) {
    if (hook.source === "user" && hook.version <= 0) {
      traces.push(traceForHook(hook, "skipped", null, input.agentId, "unpublished"));
      continue;
    }
    if (!hook.enabled) {
      traces.push(traceForHook(hook, "disabled", null, input.agentId, "disabled"));
      continue;
    }
    if (!hook.cliBindings.includes(input.agentId)) {
      traces.push(traceForHook(hook, "skipped", null, input.agentId, "unbound-cli"));
      continue;
    }
    const content = renderPromptHookTemplate(hook.templateBody ?? "", {
      agentId: input.agentId,
      sampleInput: input.sampleInput,
    });
    rendered.push(content);
    traces.push(traceForHook(hook, "fired", content, input.agentId));
  }
  const renderedContent = [...rendered, input.sampleInput].filter(Boolean).join("\n\n");
  writePromptHookTraces([...traces, ...readPromptHookTraces()]);
  return { agentId: input.agentId, renderedContent, trace: traces };
}

export function mutationToPromptHook(input: PromptHookMutationInput): PromptHook {
  validatePromptHookInput(input);
  const timestamp = nowIso();
  return {
    id: input.id,
    name: input.name.trim(),
    description: input.description.trim(),
    category: input.category,
    stage: input.stage,
    order: input.order,
    version: 1,
    source: "user",
    enabled: input.enabled,
    disableable: true,
    cliBindings: [...input.cliBindings],
    governance: input.governance,
    templateBody: input.templateBody,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}

export function findPromptHook(hookId: string) {
  const hook = listEffectivePromptHooks().find((candidate) => candidate.id === hookId);
  if (!hook) throw new Error(`Prompt Hook not found: ${hookId}`);
  return hook;
}
