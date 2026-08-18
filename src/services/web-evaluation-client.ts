import type { AgentService } from "./agent-service";
import type { EvaluationService } from "./evaluation-service";
import type { EvaluationArena, EvaluationTask } from "../types/evaluation";

const webEvaluationTasks: EvaluationTask[] = [
  { id: "fix-null-auth-token", version: 1, category: "bugfix", prompt: "Fix null authentication token handling.", timeoutSeconds: 120, verifierProfiles: ["npm-test", "static-files"] },
  { id: "add-parser-test", version: 1, category: "tests", prompt: "Add deterministic parser tests.", timeoutSeconds: 120, verifierProfiles: ["npm-test"] },
  { id: "refactor-search", version: 1, category: "refactor", prompt: "Refactor search without changing ordering.", timeoutSeconds: 180, verifierProfiles: ["cargo-test"] },
];
let webEvaluationArenas: EvaluationArena[] = [];

function webEvaluationAttempt(arenaId: string, task: EvaluationTask, agentId: string, index: number) {
  const succeeded = index === 0;
  return {
    id: `${arenaId}-attempt-${index + 1}`, arenaId, canonicalRunId: `${arenaId}-run-${index + 1}`,
    taskId: task.id, taskVersion: task.version,
    agent: { agentId, providerId: agentId === "onepiece" ? "onepiece" : "managed-cli", modelId: agentId === "onepiece" ? "mock-local" : null, interactionMode: agentId === "onepiece" ? "api" : "cli", configurationFingerprint: `mock-${agentId}-v1` },
    outcome: succeeded ? "succeeded" as const : "task_failed" as const,
    checks: [{ checkId: "deterministic-tests", passed: succeeded, summary: succeeded ? "42/42" : "41/42" }],
    metrics: [
      { name: "wall_time", value: 12 + index, unit: "seconds", quality: "reported" as const, source: "runtime" },
      { name: "tool_calls", value: 5 + index, unit: "count", quality: "reported" as const, source: "runtime" },
      { name: "input_tokens", value: index === 0 ? 820 : null, unit: "tokens", quality: index === 0 ? "reported" as const : "unavailable" as const, source: "provider" },
    ],
    contextEvidenceManifestId: index === 0 ? "mock-context-evidence-v1" : null, artifactIds: [`${arenaId}-diff-${index + 1}`],
    timeline: [
      { id: "prepare", kind: "lifecycle" as const, label: "Clean fixture prepared", status: "completed" },
      { id: "tool", kind: "tool" as const, label: "Patch applied", status: "completed" },
      { id: "verify", kind: "verification" as const, label: "Deterministic verification", status: succeeded ? "passed" : "failed" },
    ],
  };
}

export const webEvaluationClient: EvaluationService = {
  async listEvaluationTasks() { return structuredClone(webEvaluationTasks); },
  async startEvaluation(input) {
    const task = webEvaluationTasks.find((item) => item.id === input.taskId && item.version === input.taskVersion);
    if (!task || input.agentIds.length === 0 || input.agentIds.length > 8) throw new Error("Invalid evaluation configuration");
    const id = `web-eval-${webEvaluationArenas.length + 1}`;
    const arena: EvaluationArena = { id, operationId: `${id}-operation`, taskId: task.id, taskVersion: task.version, rankingVersion: "deterministic-v1", attempts: input.agentIds.map((agentId, index) => webEvaluationAttempt(id, task, agentId, index)) };
    webEvaluationArenas = [arena, ...webEvaluationArenas]; return structuredClone(arena);
  },
  async listEvaluationArenas() { return structuredClone(webEvaluationArenas); },
  async getEvaluationArena(arenaId) { const arena = webEvaluationArenas.find((item) => item.id === arenaId); if (!arena) throw new Error("Evaluation not found"); return structuredClone(arena); },
  async cancelEvaluation(this: AgentService, arenaId) { const arena = await this.getEvaluationArena(arenaId); const cancelled = { ...arena, attempts: arena.attempts.map((attempt) => ["queued", "running"].includes(attempt.outcome) ? { ...attempt, outcome: "cancelled" as const } : attempt) }; webEvaluationArenas = webEvaluationArenas.map((item) => item.id === arenaId ? cancelled : item); return structuredClone(cancelled); },
  async getEvaluationAttempt(attemptId) { for (const arena of webEvaluationArenas) { const attempt = arena.attempts.find((item) => item.id === attemptId); if (attempt) return structuredClone(attempt); } throw new Error("Evaluation attempt not found"); },
  async exportEvaluation(this: AgentService, arenaId) { return { schemaVersion: 1, arena: await this.getEvaluationArena(arenaId) }; },
};
