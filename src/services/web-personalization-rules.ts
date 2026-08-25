import type {
  AgentPersonalizationCapability,
  PersonalizationPolicy,
  PersonalizationPolicyPatch,
  PersonalizationPolicyRef,
} from "../types/personalization";
import type {
  MaintenanceResult,
  MemoryDetail,
  MemoryQuery,
  MemorySummary,
  ResetScope,
} from "../types/personalization-memory";
import { listWebMemories, policyScopeKey } from "./web-personalization-state";

export { previewFor } from "./web-personalization-preview";

/**
 * The rules the mock enforces so a screen's unhappy paths actually run.
 *
 * The messages are the native command layer's, verbatim: `CommandError` serializes to its message
 * and nothing else, so the message *is* the contract a screen matches on. A mock that invented
 * friendlier text would leave every error branch untested against what the desktop really sends.
 */

export function validation(message: string): Error {
  return new Error(`validation error: ${message}`);
}

export function conflict(expected: number, stored: number): Error {
  return new Error(`personalization-revision-conflict: expected ${expected}, stored ${stored}`);
}

export function notFound(): Error {
  return new Error("personalization-not-found");
}

/** The unit separator the native cursor joins on, chosen because neither half can contain it. */
export const CURSOR_SEPARATOR = "\u001F";

/**
 * A fixed set covering every capability combination a screen has to render.
 *
 * Deliberately not "every Agent in the mock registry": the point is that the screen builds its
 * controls from what an Agent reports, so the mock has to include one that reports `false` for
 * each of them.
 */
export const MOCK_AGENT_CAPABILITIES: AgentPersonalizationCapability[] = [
  {
    agentId: "onepiece",
    displayName: "OnePiece",
    supportsCustomInstructions: true,
    supportsMemoryIndex: true,
    supportsSelectedMemoryBodies: true,
    supportsAutomaticExtraction: true,
  },
  {
    agentId: "claude-code",
    displayName: "Claude Code",
    supportsCustomInstructions: true,
    supportsMemoryIndex: true,
    supportsSelectedMemoryBodies: false,
    supportsAutomaticExtraction: false,
  },
  {
    agentId: "codex-cli",
    displayName: "Codex CLI",
    supportsCustomInstructions: true,
    supportsMemoryIndex: false,
    supportsSelectedMemoryBodies: false,
    supportsAutomaticExtraction: false,
  },
  {
    agentId: "gemini-cli",
    displayName: "Gemini CLI",
    supportsCustomInstructions: false,
    supportsMemoryIndex: false,
    supportsSelectedMemoryBodies: false,
    supportsAutomaticExtraction: false,
  },
];

export function requireScopeKey(scope: PersonalizationPolicyRef): void {
  if (scope.scopeKind === "global") return;
  const needsAgent = scope.scopeKind !== "workspace";
  const needsWorkspace = scope.scopeKind !== "agent";
  if ((needsAgent && !scope.agentId) || (needsWorkspace && !scope.workspaceKey)) {
    throw validation(`unsupported policy scope: ${scope.scopeKind}`);
  }
}

/** An absent field leaves the stored value alone; only what the user touched is republished. */
export function applyPatch(
  current: PersonalizationPolicy,
  patch: PersonalizationPolicyPatch,
): PersonalizationPolicy {
  return {
    ...current,
    scopeKind: patch.scopeKind,
    scopeKey: policyScopeKey(patch),
    instructionMergeMode: patch.instructionMergeMode ?? current.instructionMergeMode,
    aboutUser: patch.aboutUser ?? current.aboutUser,
    styleRules: patch.styleRules ?? current.styleRules,
    memoryReadMode: patch.memoryReadMode ?? current.memoryReadMode,
    explicitSaveMode: patch.explicitSaveMode ?? current.explicitSaveMode,
    automaticExtractionMode: patch.automaticExtractionMode ?? current.automaticExtractionMode,
    globalMemoryAccessMode: patch.globalMemoryAccessMode ?? current.globalMemoryAccessMode,
  };
}

export function matchesQuery(memory: MemoryDetail, query: MemoryQuery): boolean {
  if (query.status && memory.status !== query.status) return false;
  if (!query.status && memory.status === "candidate") return false;
  if (query.memoryType && memory.memoryType !== query.memoryType) return false;
  if (query.sourceAgentId && memory.sourceAgentId !== query.sourceAgentId) return false;
  if (query.scopeKind === "global" && memory.scopeKind !== "global") return false;
  if (query.scopeKind === "workspace" && memory.workspaceKey !== query.workspaceKey) return false;
  if (query.text) {
    const needle = query.text.toLowerCase();
    const haystack = `${memory.name} ${memory.description}`.toLowerCase();
    if (!haystack.includes(needle)) return false;
  }
  return true;
}

/** Drops the body, exactly as the native page does. */
export function summarize(memory: MemoryDetail): MemorySummary {
  return {
    id: memory.id,
    name: memory.name,
    description: memory.description,
    memoryType: memory.memoryType,
    scopeKind: memory.scopeKind,
    workspaceKey: memory.workspaceKey,
    status: memory.status,
    source: memory.source,
    sensitivity: memory.audienceAgentIds ? "restricted" : memory.sensitivity,
    revision: memory.revision,
    updatedAt: memory.updatedAt,
  };
}

export function cursorIndex(matched: MemoryDetail[], cursor: string): number {
  const separator = cursor.indexOf(CURSOR_SEPARATOR);
  if (separator < 0) throw validation("unreadable page cursor");
  const id = cursor.slice(separator + 1);
  const index = matched.findIndex((memory) => memory.id === id);
  return index < 0 ? matched.length : index;
}

export function renderCursor(memory: MemoryDetail): string {
  return `${memory.updatedAt}${CURSOR_SEPARATOR}${memory.id}`;
}

/**
 * Derived from what the token authorises rather than minted at random.
 *
 * A random token would still be quoted back correctly by a well-behaved screen, so the mock would
 * accept a token issued for one scope on another scope's execute -- which is the exact mistake the
 * native check exists to catch.
 */
export function resetToken(scope: ResetScope): string {
  return `reset-${scope.scopeKind ?? "any"}-${scope.workspaceKey ?? ""}-${scope.includeArchived}`;
}

export function resetMatches(scope: ResetScope): MemoryDetail[] {
  return listWebMemories().filter((memory) => {
    if (memory.status === "archived" && !scope.includeArchived) return false;
    if (scope.scopeKind === "global") return memory.scopeKind === "global";
    if (scope.scopeKind === "workspace") return memory.workspaceKey === scope.workspaceKey;
    return true;
  });
}

export function maintenance(counts: Partial<MaintenanceResult>): MaintenanceResult {
  return {
    matched: 0,
    deletedFiles: 0,
    removedProjectionRows: 0,
    revokedRetrievalEntries: 0,
    quarantined: 0,
    failures: [],
    ...counts,
  };
}
