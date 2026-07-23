import type { LoopRun } from "../types/loop";

export const loopQueryKeys = {
  all: ["loops"] as const,
  definitions: ["loops", "definitions"] as const,
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
