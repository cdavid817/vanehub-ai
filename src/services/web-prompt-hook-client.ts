import type { PromptHookService } from "./prompt-hook-service";
import { nowIso } from "./web-mock-clock";
import {
  assemblePromptHooks,
  findPromptHook,
  isManagedCliAgentId,
  listEffectivePromptHooks,
  mutationToPromptHook,
  promptHookStats,
  readPromptHookTraces,
  readStoredPromptHooks,
  renderPromptHookTemplate,
  traceForHook,
  validatePromptHookInput,
  writePromptHookTraces,
  writeStoredPromptHooks,
} from "./web-prompt-hook-store";
import {
  deleteWebPromptHookVersionState,
  publishWebPromptHook,
  rollbackWebPromptHook,
  saveWebPromptHookDraft,
  webPromptHookHistory,
  webPromptHookVariables,
} from "./web-prompt-hook-versions";
import type { PromptHook } from "../types/prompt-hook";

export const webPromptHookClient: PromptHookService = {
  async listPromptHooks() {
    const hooks = listEffectivePromptHooks();
    return { hooks, stats: promptHookStats(hooks) };
  },

  async createPromptHook(input) {
    const stored = readStoredPromptHooks();
    if (listEffectivePromptHooks().some((hook) => hook.id === input.id)) {
      throw new Error(`Prompt Hook already exists: ${input.id}`);
    }
    const created = mutationToPromptHook(input);
    const hook: PromptHook = {
      ...created,
      version: 0,
      publishedVersion: null,
      hasDraft: true,
      draftRevision: 1,
      enabled: false,
    };
    writeStoredPromptHooks({ ...stored, [hook.id]: hook });
    saveWebPromptHookDraft({
      hookId: hook.id,
      expectedRevision: null,
      draft: input,
    });
    return hook;
  },

  async updatePromptHook(hookId, input) {
    const current = findPromptHook(hookId);
    if (current.source === "builtin") {
      throw new Error("Built-in Prompt Hook content cannot be edited");
    }
    if (input.id !== hookId) {
      throw new Error("Prompt Hook id cannot be changed");
    }
    validatePromptHookInput(input);
    const history = webPromptHookHistory(current);
    const draft = saveWebPromptHookDraft({
      hookId,
      expectedRevision: history.draft?.revision ?? null,
      draft: input,
    });
    const updated: PromptHook = {
      ...current,
      hasDraft: true,
      draftRevision: draft.revision,
    };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [hookId]: updated });
    return updated;
  },

  async deletePromptHook(hookId) {
    const current = findPromptHook(hookId);
    if (current.source === "builtin") {
      throw new Error("Built-in Prompt Hook cannot be deleted");
    }
    const stored = { ...readStoredPromptHooks() };
    delete stored[hookId];
    writeStoredPromptHooks(stored);
    deleteWebPromptHookVersionState(hookId);
  },

  async setPromptHookEnabled(hookId, enabled) {
    const current = findPromptHook(hookId);
    if (!enabled && !current.disableable) {
      throw new Error("Prompt Hook cannot be disabled");
    }
    const updated = { ...current, enabled, updatedAt: nowIso() };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [hookId]: updated });
    return updated;
  },

  async setPromptHookCliBindings(hookId, agentIds) {
    if (!agentIds.every(isManagedCliAgentId)) throw new Error("Unsupported Prompt Hook CLI binding");
    const current = findPromptHook(hookId);
    const cliBindings = Array.from(new Set(agentIds));
    const updated = { ...current, cliBindings, updatedAt: nowIso() };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [hookId]: updated });
    return updated;
  },

  async previewPromptHook(input) {
    const hook = findPromptHook(input.hookId);
    const sampleInput = input.sampleInput ?? "Preview request";
    const renderedContent = renderPromptHookTemplate(hook.templateBody ?? "", {
      agentId: input.agentId,
      sampleInput,
    });
    const trace = [traceForHook(hook, hook.enabled ? "fired" : "disabled", hook.enabled ? renderedContent : null, input.agentId, hook.enabled ? undefined : "disabled")];
    writePromptHookTraces([...trace, ...readPromptHookTraces()]);
    return { hookId: hook.id, agentId: input.agentId, renderedContent, trace };
  },

  async previewPromptAssembly(input) {
    return assemblePromptHooks(input);
  },

  async listPromptHookTraces(limit = 25) {
    return readPromptHookTraces().slice(0, limit);
  },

  async listPromptHookVariables() {
    return webPromptHookVariables.map((variable) => ({ ...variable, aliases: [...variable.aliases] }));
  },

  async savePromptHookDraft(input) {
    const current = findPromptHook(input.hookId);
    if (current.source === "builtin") throw new Error("Built-in Prompt Hook content cannot be edited");
    return saveWebPromptHookDraft(input);
  },

  async publishPromptHook(input) {
    const current = findPromptHook(input.hookId);
    if (current.source === "builtin") throw new Error("Built-in Prompt Hook content cannot be edited");
    const result = publishWebPromptHook(input, current);
    const updated: PromptHook = {
      ...current,
      ...result.published,
      version: result.version.version,
      publishedVersion: result.version.version,
      hasDraft: false,
      draftRevision: null,
      updatedAt: result.version.publishedAt,
    };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [current.id]: updated });
    return result.version;
  },

  async getPromptHookVersionHistory(hookId) {
    const current = findPromptHook(hookId);
    if (current.source === "builtin") {
      return {
        hookId,
        publishedVersion: current.version,
        draft: null,
        versions: [{
          hookId,
          version: current.version,
          contentHash: `builtin-${hookId}-${current.version}`,
          publicationKind: "publish",
          rollbackFromVersion: null,
          publishedAt: current.updatedAt,
        }],
        evaluations: [],
      };
    }
    return webPromptHookHistory(current);
  },

  async rollbackPromptHook(input) {
    const current = findPromptHook(input.hookId);
    if (current.source === "builtin") throw new Error("Built-in Prompt Hook content cannot be edited");
    const result = rollbackWebPromptHook(input, current);
    const history = webPromptHookHistory(current);
    const updated: PromptHook = {
      ...current,
      ...result.published,
      version: result.version.version,
      publishedVersion: result.version.version,
      hasDraft: history.draft !== null,
      draftRevision: history.draft?.revision ?? null,
      updatedAt: result.version.publishedAt,
    };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [current.id]: updated });
    return result.version;
  },
};
