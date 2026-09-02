import type { LoopDefinition, LoopRun } from "../types/loop";

export const loopQueryKeys = {
  all: ["loops"] as const,
  definitions: ["loops", "definitions"] as const,
  projects: ["loops", "projects"] as const,
  branches: (projectPath: string) => ["loops", "branches", projectPath] as const,
  readiness: (definitionId: string) => ["loops", "readiness", definitionId] as const,
  runs: (definitionId?: string) => ["loops", "runs", definitionId ?? null] as const,
  run: (runId: string) => ["loops", "run", runId] as const,
};

export function preserveLoopRuns(previous: LoopRun[] | undefined) {
  return previous;
}

export function applyLoopRunUpdate(current: LoopRun[] | undefined, updated: LoopRun) {
  if (!current) return current;
  const index = current.findIndex((run) => run.id === updated.id);
  if (index < 0) return current;
  const next = [...current];
  next[index] = updated;
  return next;
}

// The three helpers below let `use-loop-mutations.ts` patch `loopQueryKeys.definitions` directly
// (task 17.14) instead of `invalidateQueries` + a whole-collection refetch, which swaps every
// row's object identity and made an unrelated definition's row flicker/reload for one row's own
// edit. Same shape as `applyLoopRunUpdate` above: return the input unchanged (including a still
// -unfetched `undefined`) when there is nothing local to patch.

export function applyLoopDefinitionUpdate(current: LoopDefinition[] | undefined, updated: LoopDefinition) {
  if (!current) return current;
  const index = current.findIndex((definition) => definition.id === updated.id);
  if (index < 0) return current;
  const next = [...current];
  next[index] = updated;
  return next;
}

// Both backends sort the definitions list by `updatedAt` descending, and a just-created row is
// always the most recent, so it belongs at the front.
export function insertLoopDefinition(current: LoopDefinition[] | undefined, created: LoopDefinition) {
  if (!current) return current;
  return [created, ...current];
}

export function removeLoopDefinition(current: LoopDefinition[] | undefined, definitionId: string) {
  if (!current) return current;
  return current.filter((definition) => definition.id !== definitionId);
}
