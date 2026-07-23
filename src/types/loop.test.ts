import { describe, expect, it } from "vitest";
import type {
  LoopEventKind,
  LoopEvidenceKind,
  LoopEvidenceStatus,
  LoopInspectionSurface,
  LoopRole,
  LoopRunPhase,
  LoopRunStatus,
  LoopTerminalReason,
  LoopVerifierRecommendation,
} from "./loop";

function exactValues<Union>() {
  return <Values extends readonly Union[]>(
    values: Exclude<Union, Values[number]> extends never ? Values : never,
  ) => values;
}

const runStatuses = exactValues<LoopRunStatus>()([
  "queued", "running", "paused", "awaiting-acceptance", "succeeded", "failed", "cancelled",
] as const);
const runPhases = exactValues<LoopRunPhase>()([
  "preparing", "acting", "verifying", "deciding", "finalizing",
] as const);
const terminalReasons = exactValues<LoopTerminalReason>()([
  "goal-met", "max-iterations", "time-budget", "phase-timeout", "runtime-errors", "no-progress",
  "verification-failed", "verifier-blocked", "runtime-error", "recovery-required", "user-rejected", "user-stopped",
] as const);
const roles = exactValues<LoopRole>()(["worker", "verifier"] as const);
const recommendations = exactValues<LoopVerifierRecommendation>()(["pass", "revise", "blocked"] as const);
const evidenceKinds = exactValues<LoopEvidenceKind>()([
  "worktree", "worker", "verification", "verifier", "decision", "recovery",
] as const);
const evidenceStatuses = exactValues<LoopEvidenceStatus>()([
  "pending", "passed", "failed", "blocked", "cancelled",
] as const);
const eventKinds = exactValues<LoopEventKind>()([
  "run-updated", "iteration-updated", "evidence-added",
] as const);
const inspectionSurfaces = exactValues<LoopInspectionSurface>()([
  "chat", "changes", "files", "terminal", "logs", "report", "usage",
] as const);

describe("Loop frontend model", () => {
  it("keeps every discriminated value explicit and exhaustive", () => {
    expect({
      runStatuses,
      runPhases,
      terminalReasons,
      roles,
      recommendations,
      evidenceKinds,
      evidenceStatuses,
      eventKinds,
      inspectionSurfaces,
    }).toEqual({
      runStatuses: ["queued", "running", "paused", "awaiting-acceptance", "succeeded", "failed", "cancelled"],
      runPhases: ["preparing", "acting", "verifying", "deciding", "finalizing"],
      terminalReasons: [
        "goal-met", "max-iterations", "time-budget", "phase-timeout", "runtime-errors", "no-progress",
        "verification-failed", "verifier-blocked", "runtime-error", "recovery-required", "user-rejected", "user-stopped",
      ],
      roles: ["worker", "verifier"],
      recommendations: ["pass", "revise", "blocked"],
      evidenceKinds: ["worktree", "worker", "verification", "verifier", "decision", "recovery"],
      evidenceStatuses: ["pending", "passed", "failed", "blocked", "cancelled"],
      eventKinds: ["run-updated", "iteration-updated", "evidence-added"],
      inspectionSurfaces: ["chat", "changes", "files", "terminal", "logs", "report", "usage"],
    });
  });
});
