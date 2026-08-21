import type { LoopEvidence, LoopIteration, LoopRun } from "../types/loop";

export type LoopCheckOutcome = "passed" | "failed" | "blocked" | "pending" | "cancelled" | "not-evaluated";

export interface LoopBudgetSummary {
  elapsedMs: number;
  remainingMs: number;
  consumedPercent: number;
  exhausted: boolean;
}

export interface LoopChangeStatistics {
  changedFiles: number;
  additions: number;
  deletions: number;
}

export interface LoopIterationComparison {
  resolvedFailures: string[];
  newFailures: string[];
  changeDelta: LoopChangeStatistics | null;
}

export function selectCurrentLoopActivity(run: LoopRun): string | null {
  const evidence = [...run.iterations].reverse().flatMap((iteration) => [...iteration.evidence].reverse());
  return evidence.find((item) => item.status === "pending")?.summary
    ?? evidence[0]?.summary
    ?? (run.status === "queued" ? "preparing" : null);
}

export function selectLoopBudget(run: LoopRun, nowMs: number): LoopBudgetSummary {
  const start = Date.parse(run.startedAt ?? run.createdAt);
  const terminal = Date.parse(run.completedAt ?? run.updatedAt);
  const active = ["queued", "running", "paused", "awaiting-acceptance"].includes(run.status);
  const elapsedMs = Math.max(0, (active ? nowMs : terminal) - start);
  const totalMs = run.definitionSnapshot.limits.totalTimeoutSeconds * 1_000;
  return {
    elapsedMs,
    remainingMs: Math.max(0, totalMs - elapsedMs),
    consumedPercent: totalMs === 0 ? 100 : Math.min(100, Math.round((elapsedMs / totalMs) * 100)),
    exhausted: elapsedMs >= totalMs,
  };
}

export function selectLatestDecision(run: LoopRun) {
  return [...run.iterations].reverse().find((iteration) => iteration.decisionReason)?.decisionReason ?? null;
}

export function selectRequiredCheckOutcomes(run: LoopRun) {
  const latest = run.iterations.at(-1);
  return run.definitionSnapshot.verificationCommands.filter((command) => command.required).map((command) => {
    const evidence = latest?.evidence.find((item) => item.kind === "verification" && item.commandId === command.id);
    return { commandId: command.id, outcome: evidence ? evidence.status : "not-evaluated" as LoopCheckOutcome };
  });
}

export function selectChangeStatistics(iteration: LoopIteration): LoopChangeStatistics | null {
  const evidence = iteration.evidence.find((item) => item.kind === "worker" && item.details);
  if (!evidence) return null;
  const changedFiles = detailNumber(evidence, "changedFiles");
  const additions = detailNumber(evidence, "additions");
  const deletions = detailNumber(evidence, "deletions");
  if (changedFiles === null || additions === null || deletions === null) return null;
  return { changedFiles, additions, deletions };
}

export function compareConsecutiveIterations(
  previous: LoopIteration,
  current: LoopIteration,
): LoopIterationComparison {
  const previousFailures = failedCommandIds(previous);
  const currentFailures = failedCommandIds(current);
  const previousChanges = selectChangeStatistics(previous);
  const currentChanges = selectChangeStatistics(current);
  return {
    resolvedFailures: [...previousFailures].filter((id) => !currentFailures.has(id)).sort(),
    newFailures: [...currentFailures].filter((id) => !previousFailures.has(id)).sort(),
    changeDelta: previousChanges && currentChanges ? {
      changedFiles: currentChanges.changedFiles - previousChanges.changedFiles,
      additions: currentChanges.additions - previousChanges.additions,
      deletions: currentChanges.deletions - previousChanges.deletions,
    } : null,
  };
}

export function selectRecoveryGuidance(run: LoopRun): "resume" | "inspect" | "none" {
  if (run.terminalReason === "recovery-required") return "inspect";
  if (run.status === "paused") return "resume";
  return "none";
}

function failedCommandIds(iteration: LoopIteration) {
  return new Set(iteration.evidence.filter((item) => (
    item.kind === "verification" && ["failed", "blocked"].includes(item.status) && item.commandId
  )).map((item) => item.commandId as string));
}

function detailNumber(evidence: LoopEvidence, key: string) {
  const value = evidence.details?.[key];
  return typeof value === "number" ? value : null;
}
