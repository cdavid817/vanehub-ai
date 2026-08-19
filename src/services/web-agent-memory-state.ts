import type { AgentMemory, AgentMemoryType } from "../types/agent";
import { nowIso } from "./web-mock-clock";

/** Mock cross-session memories (`add-agent-cross-session-memory`, extended to CLI-wrapped agents
 * by `add-cli-memory-support`) — a single host-level pool shared by every agent kind, matching
 * the real backend's shared-pool model. Starts empty; real memories only ever come from a
 * `remember` tool call or extraction, both simulated in `sendMessage`.
 *
 * Owned here and never exported. An exported mutable binding re-imported from two modules gives
 * two divergent copies of the mock world, which surfaces as one UI panel showing stale data while
 * another shows fresh. Callers reach the pool through the accessors below. */
let webAgentMemories: AgentMemory[] = [];
let nextAgentMemoryId = 1;

/** Mirrors the native store's deterministic derivation: a writer that supplies no name gets one
 * from the content, and only ASCII survives slugging, so non-Latin content falls back to the
 * sequence number rather than producing an empty stem. */
function deriveMemoryName(content: string, sequence: number): string {
  const slug = content
    .split(/[^a-zA-Z0-9]+/u)
    .filter(Boolean)
    .slice(0, 6)
    .join("-")
    .toLowerCase();
  return slug || `memory-${sequence}`;
}

/** Mirrors the native bounds so the mock truncates where the desktop runtime would. */
const WEB_MEMORY_INDEX_LINE_CAP = 200;

/** Mirrors the native selection bound. */
const WEB_MEMORY_SELECTION_CAP = 5;

/** Returns the live array, matching what a direct read of the binding returned. */
export function listWebAgentMemories(): AgentMemory[] {
  return webAgentMemories;
}

export function deleteWebAgentMemory(memoryId: string): void {
  webAgentMemories = webAgentMemories.filter((memory) => memory.id !== memoryId);
}

export function clearWebAgentMemories(): void {
  webAgentMemories = [];
}

/**
 * Simulates one generation's memory injection: an index over the whole pool, plus the handful of
 * bodies a selection would have read in full.
 *
 * Deterministic rather than model-driven — the Web runtime must reproduce the desktop's observable
 * shape without issuing a provider request, so it stands in for the selector with recency, which
 * is the same ordering the index already uses.
 */
export function simulateWebMemoryIndexInjection(): { indexed: number; selected: AgentMemory[] } {
  const indexed = Math.min(webAgentMemories.length, WEB_MEMORY_INDEX_LINE_CAP);
  return {
    indexed,
    selected: webAgentMemories.slice(0, Math.min(indexed, WEB_MEMORY_SELECTION_CAP)),
  };
}

function disambiguateMemoryName(base: string): string {
  if (!webAgentMemories.some((memory) => memory.name === base)) {
    return base;
  }
  let suffix = 2;
  while (webAgentMemories.some((memory) => memory.name === `${base}-${suffix}`)) {
    suffix += 1;
  }
  return `${base}-${suffix}`;
}

export function createWebAgentMemory(
  agentId: string,
  folder: string | null,
  content: string,
  source: AgentMemory["source"],
  metadata: { name?: string; description?: string; memoryType?: AgentMemoryType | null } = {},
): AgentMemory {
  // An explicit name addresses an existing memory, so it is used as-is and replaces. A derived one
  // must not: two agents recording the same fact are two memories in the shared pool, so the
  // native store's collision suffix is mirrored here rather than silently merging them.
  const name = metadata.name ?? disambiguateMemoryName(deriveMemoryName(content, nextAgentMemoryId));
  const memory: AgentMemory = {
    // The native store's identity is the file path, so the mock mirrors that shape rather than
    // inventing an opaque id the management view would have to special-case.
    id: `${name}.md`,
    agentId,
    folder,
    name,
    description: metadata.description ?? content.split("\n")[0] ?? content,
    memoryType: metadata.memoryType ?? null,
    content,
    source,
    createdAt: nowIso(),
  };
  nextAgentMemoryId += 1;
  // Saving under an existing name replaces that memory, matching the native store's update path.
  webAgentMemories = [memory, ...webAgentMemories.filter((existing) => existing.name !== name)];
  return memory;
}

/** `add-cli-memory-support`: the shared memory pool is no longer isolated per agent id, so tests
 * that seed memories can leak into later tests within the same file unless explicitly cleared. */
export function resetWebAgentMemoriesForTest() {
  webAgentMemories = [];
}
