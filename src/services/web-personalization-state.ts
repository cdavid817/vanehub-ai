import type { PersonalizationPolicy, PersonalizationPolicyRef } from "../types/personalization";
import type { MemoryCandidate, MemoryDetail } from "../types/personalization-memory";
import { daysAgoIso } from "./web-mock-clock";

/**
 * The mock personalization world (`add-unified-personalization-governance`).
 *
 * Owned here and never exported as a mutable binding: two modules re-importing one would each get
 * their own copy, which shows up as one panel rendering stale policy while another renders fresh.
 *
 * Everything is deterministic. Ids count up, revisions count up, and the reset token is derived
 * from what it authorises -- a mock that minted random tokens would let a screen pass a token from
 * one preview into another scope's execute and never notice.
 */

/** The composite key a scope stores under. Mirrors the native `scope_key`. */
export function policyScopeKey(scope: PersonalizationPolicyRef): string {
  if (scope.scopeKind === "global") return "";
  if (scope.scopeKind === "agent") return scope.agentId ?? "";
  if (scope.scopeKind === "workspace") return scope.workspaceKey ?? "";
  return `${scope.workspaceKey ?? ""}::${scope.agentId ?? ""}`;
}

function inheritedPolicy(scope: PersonalizationPolicyRef): PersonalizationPolicy {
  return {
    scopeKind: scope.scopeKind,
    scopeKey: policyScopeKey(scope),
    revision: 0,
    instructionMergeMode: "inherit",
    aboutUser: "",
    styleRules: "",
    memoryReadMode: "inherit",
    explicitSaveMode: "inherit",
    automaticExtractionMode: "inherit",
    globalMemoryAccessMode: "inherit",
  };
}

const policies = new Map<string, PersonalizationPolicy>([
  [
    storageKey({ scopeKind: "global" }),
    {
      scopeKind: "global",
      scopeKey: "",
      revision: 3,
      instructionMergeMode: "append",
      aboutUser: "Works on a desktop Tauri app; prefers metric units.",
      styleRules: "Answer in the language the question was asked in.",
      memoryReadMode: "enabled",
      explicitSaveMode: "enabled",
      automaticExtractionMode: "enabled",
      globalMemoryAccessMode: "enabled",
    },
  ],
]);

function storageKey(scope: PersonalizationPolicyRef): string {
  return `${scope.scopeKind}\u001F${policyScopeKey(scope)}`;
}

export function readWebPolicy(scope: PersonalizationPolicyRef): PersonalizationPolicy | null {
  return policies.get(storageKey(scope)) ?? null;
}

export function listWebPolicies(): PersonalizationPolicy[] {
  return [...policies.values()];
}

export function writeWebPolicy(
  scope: PersonalizationPolicyRef,
  apply: (current: PersonalizationPolicy) => PersonalizationPolicy,
): PersonalizationPolicy {
  const current = policies.get(storageKey(scope)) ?? inheritedPolicy(scope);
  const next = { ...apply(current), revision: current.revision + 1 };
  policies.set(storageKey(scope), next);
  return next;
}

const memories = new Map<string, MemoryDetail>([
  [
    "mem-0000000000000001",
    {
      id: "mem-0000000000000001",
      name: "prefers-metric-units",
      description: "Uses metric units in explanations.",
      memoryType: "user",
      content: "The user prefers metric units and 24-hour time.",
      scopeKind: "global",
      workspaceKey: null,
      audienceAgentIds: null,
      status: "active",
      source: "explicit_user",
      sensitivity: "normal",
      revision: 1,
      sourceAgentId: null,
      createdAt: daysAgoIso(30),
      updatedAt: daysAgoIso(30),
    },
  ],
  [
    "mem-0000000000000002",
    {
      id: "mem-0000000000000002",
      name: "vanehub-uses-npm",
      description: "This project pins npm, not pnpm.",
      memoryType: "project",
      content: "VaneHub AI uses npm; package-lock.json is authoritative.",
      scopeKind: "workspace",
      workspaceKey: "ws-vanehub",
      audienceAgentIds: null,
      status: "active",
      source: "onepiece_automatic",
      sensitivity: "normal",
      revision: 2,
      sourceAgentId: "onepiece",
      createdAt: daysAgoIso(12),
      updatedAt: daysAgoIso(4),
    },
  ],
  [
    "mem-0000000000000003",
    {
      id: "mem-0000000000000003",
      name: "old-review-preference",
      description: "Superseded review preference.",
      memoryType: "feedback",
      content: "Prefers short review summaries.",
      scopeKind: "global",
      workspaceKey: null,
      audienceAgentIds: ["claude-code"],
      status: "archived",
      source: "cli_automatic",
      sensitivity: "normal",
      revision: 1,
      sourceAgentId: "claude-code",
      createdAt: daysAgoIso(90),
      updatedAt: daysAgoIso(60),
    },
  ],
]);

let nextMemorySequence = 4;

export function nextWebMemoryId(): string {
  const id = `mem-${String(nextMemorySequence).padStart(16, "0")}`;
  nextMemorySequence += 1;
  return id;
}

export function readWebMemory(memoryId: string): MemoryDetail | null {
  return memories.get(memoryId) ?? null;
}

/** Sorted by `updatedAt` then id, which is the order the native cursor pages through. */
export function listWebMemories(): MemoryDetail[] {
  return [...memories.values()].sort((left, right) => {
    if (left.updatedAt !== right.updatedAt) return right.updatedAt.localeCompare(left.updatedAt);
    return left.id.localeCompare(right.id);
  });
}

export function putWebMemory(memory: MemoryDetail): void {
  memories.set(memory.id, memory);
}

export function removeWebMemory(memoryId: string): boolean {
  return memories.delete(memoryId);
}

const candidates = new Map<string, MemoryCandidate>([
  [
    "cnd-0000000000000001",
    {
      id: "cnd-0000000000000001",
      kind: "create",
      name: "prefers-vitest-watch",
      description: "Runs vitest in watch mode while iterating.",
      memoryType: "feedback",
      content: "The user keeps `vitest --watch` running during a change.",
      targetId: null,
      expectedTargetRevision: null,
      source: "onepiece_automatic",
      sourceAgentId: "onepiece",
      sourceSessionId: "session-1",
      sourceMessageId: "message-8",
      createdAt: daysAgoIso(1),
    },
  ],
  [
    "cnd-0000000000000002",
    {
      id: "cnd-0000000000000002",
      kind: "update",
      name: null,
      description: null,
      memoryType: null,
      content: "VaneHub AI uses npm exclusively; pnpm breaks the katex chunk split.",
      targetId: "mem-0000000000000002",
      expectedTargetRevision: 2,
      source: "cli_automatic",
      sourceAgentId: "claude-code",
      sourceSessionId: "session-2",
      sourceMessageId: null,
      createdAt: daysAgoIso(1),
    },
  ],
]);

export function listWebCandidates(): MemoryCandidate[] {
  return [...candidates.values()].sort((left, right) => left.id.localeCompare(right.id));
}

export function readWebCandidate(candidateId: string): MemoryCandidate | null {
  return candidates.get(candidateId) ?? null;
}

export function removeWebCandidate(candidateId: string): void {
  candidates.delete(candidateId);
}

export function webPendingCandidateCount(): number {
  return candidates.size;
}
