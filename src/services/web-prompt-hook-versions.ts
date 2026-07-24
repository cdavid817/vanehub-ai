import type {
  PromptHook,
  PromptHookDraft,
  PromptHookEvaluationSummary,
  PromptHookMutationInput,
  PromptHookVariableDefinition,
  PromptHookVersion,
  PromptHookVersionHistory,
  PublishPromptHookInput,
  RollbackPromptHookInput,
  SavePromptHookDraftInput,
} from "../types/prompt-hook";

interface StoredVersion extends PromptHookVersion {
  snapshot: PromptHookMutationInput;
}

interface VersionState {
  drafts: Record<string, PromptHookDraft>;
  versions: Record<string, StoredVersion[]>;
}

const storageKey = "vanehub:web-prompt-hook-versions";
let memoryState: VersionState = { drafts: {}, versions: {} };

export const webPromptHookVariables: PromptHookVariableDefinition[] = [
  definition("agent_id", "promptHooks.variables.agentId", "promptHooks.variables.availability.agent", "codex-cli", ["agentId"]),
  definition("agent_name", "promptHooks.variables.agentName", "promptHooks.variables.availability.agent", "Codex CLI"),
  definition("current_time", "promptHooks.variables.currentTime", "promptHooks.variables.availability.invocation", "2026-07-23T12:00:00Z"),
  definition("sample_input", "promptHooks.variables.sampleInput", "promptHooks.variables.availability.input", "Review the current change.", ["sampleInput"]),
  definition("session_id", "promptHooks.variables.sessionId", "promptHooks.variables.availability.session", "session-preview"),
];

function definition(
  name: PromptHookVariableDefinition["name"],
  descriptionKey: string,
  availabilityKey: string,
  example: string,
  aliases: string[] = [],
): PromptHookVariableDefinition {
  return { name, token: `{{${name}}}`, descriptionKey, availabilityKey, example, aliases };
}

function readState(): VersionState {
  if (typeof localStorage === "undefined") return memoryState;
  const raw = localStorage.getItem(storageKey);
  if (!raw) return memoryState;
  try {
    return JSON.parse(raw) as VersionState;
  } catch {
    return memoryState;
  }
}

function writeState(state: VersionState) {
  memoryState = state;
  if (typeof localStorage !== "undefined") localStorage.setItem(storageKey, JSON.stringify(state));
}

function snapshot(hook: PromptHook): PromptHookMutationInput {
  return {
    id: hook.id,
    name: hook.name,
    description: hook.description,
    category: hook.category,
    stage: hook.stage,
    order: hook.order,
    templateBody: hook.templateBody ?? "",
    enabled: hook.enabled,
    cliBindings: [...hook.cliBindings],
    governance: { ...hook.governance },
  };
}

function hash(input: PromptHookMutationInput) {
  let value = 2166136261;
  const text = JSON.stringify(input);
  for (let index = 0; index < text.length; index += 1) {
    value = Math.imul(value ^ text.charCodeAt(index), 16777619);
  }
  return `web-${(value >>> 0).toString(16).padStart(8, "0")}`;
}

function referencedVariables(template: string) {
  return [...template.matchAll(/\{\{\s*([^{}]+?)\s*\}\}/g)].map((match) => match[1]);
}

export function validateWebPromptHookVariables(template: string) {
  const supported = new Set(webPromptHookVariables.flatMap((item) => [item.name, ...item.aliases]));
  const unknown = [...new Set(referencedVariables(template).filter((name) => !supported.has(name)))].sort();
  if (unknown.length > 0) throw new Error(`Unsupported Prompt Hook variables: ${unknown.join(", ")}`);
}

export function renderWebPromptHookTemplate(
  template: string,
  context: {
    agentId: string;
    agentName: string;
    currentTime: string;
    sampleInput: string;
    sessionId: string;
  },
) {
  validateWebPromptHookVariables(template);
  const values: Record<string, string> = {
    agent_id: context.agentId,
    agent_name: context.agentName,
    current_time: context.currentTime,
    sample_input: context.sampleInput,
    session_id: context.sessionId,
    agentId: context.agentId,
    sampleInput: context.sampleInput,
  };
  return Object.entries(values).reduce(
    (rendered, [name, value]) => rendered.replaceAll(`{{${name}}}`, value),
    template,
  );
}

export function ensureWebPromptHookVersion(hook: PromptHook) {
  if (hook.source !== "user" || hook.version <= 0) return;
  const state = readState();
  const versions = state.versions[hook.id] ?? [];
  if (versions.some((version) => version.version === hook.version)) return;
  const input = snapshot(hook);
  const version: StoredVersion = {
    hookId: hook.id,
    version: hook.version,
    contentHash: hash(input),
    publicationKind: "publish",
    rollbackFromVersion: null,
    publishedAt: hook.updatedAt,
    snapshot: input,
  };
  writeState({ ...state, versions: { ...state.versions, [hook.id]: [version, ...versions] } });
}

export function saveWebPromptHookDraft(input: SavePromptHookDraftInput): PromptHookDraft {
  const state = readState();
  const current = state.drafts[input.hookId];
  if ((input.expectedRevision ?? null) !== (current?.revision ?? null)) {
    throw new Error("Prompt Hook draft revision is stale");
  }
  const timestamp = new Date().toISOString();
  const draft: PromptHookDraft = {
    hookId: input.hookId,
    revision: (current?.revision ?? 0) + 1,
    input: { ...input.draft, cliBindings: [...input.draft.cliBindings] },
    createdAt: current?.createdAt ?? timestamp,
    updatedAt: timestamp,
  };
  writeState({ ...state, drafts: { ...state.drafts, [input.hookId]: draft } });
  return draft;
}

export function publishWebPromptHook(
  input: PublishPromptHookInput,
  current: PromptHook,
): { version: PromptHookVersion; published: PromptHookMutationInput } {
  ensureWebPromptHookVersion(current);
  const state = readState();
  const draft = state.drafts[input.hookId];
  if (!draft || draft.revision !== input.expectedDraftRevision) {
    throw new Error("Prompt Hook draft revision is stale");
  }
  const versions = state.versions[input.hookId] ?? [];
  const publishedVersion = versions[0]?.version ?? (current.version > 0 ? current.version : 0);
  if ((input.expectedPublishedVersion ?? null) !== (publishedVersion || null)) {
    throw new Error("Prompt Hook published version is stale");
  }
  validateWebPromptHookVariables(draft.input.templateBody);
  const version: StoredVersion = {
    hookId: input.hookId,
    version: publishedVersion + 1,
    contentHash: hash(draft.input),
    publicationKind: "publish",
    rollbackFromVersion: null,
    publishedAt: new Date().toISOString(),
    snapshot: draft.input,
  };
  const drafts = { ...state.drafts };
  delete drafts[input.hookId];
  writeState({
    drafts,
    versions: { ...state.versions, [input.hookId]: [version, ...versions] },
  });
  return { version: publicVersion(version), published: draft.input };
}

export function rollbackWebPromptHook(
  input: RollbackPromptHookInput,
  current: PromptHook,
): { version: PromptHookVersion; published: PromptHookMutationInput } {
  ensureWebPromptHookVersion(current);
  const state = readState();
  const versions = state.versions[input.hookId] ?? [];
  const currentVersion = versions[0]?.version ?? current.version;
  if ((input.expectedPublishedVersion ?? null) !== (currentVersion || null)) {
    throw new Error("Prompt Hook published version is stale");
  }
  const target = versions.find((version) => version.version === input.version);
  if (!target) throw new Error(`Prompt Hook version not found: ${input.version}`);
  const version: StoredVersion = {
    ...target,
    version: currentVersion + 1,
    contentHash: hash(target.snapshot),
    publicationKind: "rollback",
    rollbackFromVersion: target.version,
    publishedAt: new Date().toISOString(),
  };
  writeState({ ...state, versions: { ...state.versions, [input.hookId]: [version, ...versions] } });
  return { version: publicVersion(version), published: target.snapshot };
}

export function webPromptHookHistory(hook: PromptHook): PromptHookVersionHistory {
  ensureWebPromptHookVersion(hook);
  const state = readState();
  const stored = state.versions[hook.id] ?? [];
  return {
    hookId: hook.id,
    publishedVersion: stored[0]?.version ?? (hook.version > 0 ? hook.version : null),
    draft: state.drafts[hook.id] ?? null,
    versions: stored.map(publicVersion),
    evaluations: stored.map(mockEvaluation),
  };
}

export function deleteWebPromptHookVersionState(hookId: string) {
  const state = readState();
  const drafts = { ...state.drafts };
  const versions = { ...state.versions };
  delete drafts[hookId];
  delete versions[hookId];
  writeState({ drafts, versions });
}

function publicVersion(version: StoredVersion): PromptHookVersion {
  return {
    hookId: version.hookId,
    version: version.version,
    contentHash: version.contentHash,
    publicationKind: version.publicationKind,
    rollbackFromVersion: version.rollbackFromVersion,
    publishedAt: version.publishedAt,
    templateBody: version.templateBody,
  };
}

function mockEvaluation(version: StoredVersion): PromptHookEvaluationSummary {
  const succeededCount = version.version + 2;
  const failedCount = version.version % 2;
  const evaluated = succeededCount + failedCount;
  return {
    hookId: version.hookId,
    version: version.version,
    executionCount: evaluated + 1,
    succeededCount,
    failedCount,
    cancelledCount: 1,
    successRate: evaluated === 0 ? null : succeededCount / evaluated,
    averageElapsedMs: 900 + version.version * 125,
    minimumElapsedMs: 650,
    maximumElapsedMs: 1600 + version.version * 100,
  };
}
