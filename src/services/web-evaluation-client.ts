import type { AgentService } from "./agent-service";
import type { EvaluationService } from "./evaluation-service";
import type { EvaluationArena, EvaluationArenaPage, EvaluationArenaQuery, EvaluationTask } from "../types/evaluation";

// 18.6: mirrors `mission_control.rs`'s own `DEFAULT_LIMIT`/`MAX_LIMIT` (20/50), and
// `web-mission-control-client.ts`'s own identical `page()` helper below -- same cursor-is-really-
// just-the-offset shape, same clamp, so a reader who already knows one pagination surface in this
// app recognizes the other.
const DEFAULT_EVALUATION_PAGE_LIMIT = 20;
const MAX_EVALUATION_PAGE_LIMIT = 50;

const webEvaluationTasks: EvaluationTask[] = [
  { id: "fix-null-auth-token", version: 1, category: "bugfix", prompt: "Fix null authentication token handling.", timeoutSeconds: 120, verifierProfiles: ["npm-test", "static-files"] },
  { id: "add-parser-test", version: 1, category: "tests", prompt: "Add deterministic parser tests.", timeoutSeconds: 120, verifierProfiles: ["npm-test"] },
  { id: "refactor-search", version: 1, category: "refactor", prompt: "Refactor search without changing ordering.", timeoutSeconds: 180, verifierProfiles: ["cargo-test"] },
];
let webEvaluationArenas: EvaluationArena[] = [];

// Mirrors the native diagnostic an attempt carries when its Agent could not be dispatched
// (evaluation_api.rs `DISPATCH_CHECK_ID`), including the exact-match safe reason. The mock has no
// Agent to fail, so it exercises the shape rather than the failure: a browser session that never
// renders this branch is how the desktop detail pane and the mock drift apart.
const WEB_DISPATCH_DIAGNOSTIC = { checkId: "agent-dispatch", passed: false, summary: "evaluation Agent is not installed and available" };

function webEvaluationAttempt(arenaId: string, task: EvaluationTask, agentId: string, index: number) {
  const succeeded = index === 0;
  // The third Agent onwards never gets dispatched, so an arena wide enough to include one shows
  // the failure the desktop client shows: a verdict, a reason, and no metrics to read.
  if (index >= 2) return webDispatchFailedAttempt(arenaId, task, agentId, index);
  return {
    id: `${arenaId}-attempt-${index + 1}`, arenaId, canonicalRunId: `${arenaId}-run-${index + 1}`,
    taskId: task.id, taskVersion: task.version,
    agent: { agentId, providerId: agentId === "onepiece" ? "onepiece" : "managed-cli", modelId: agentId === "onepiece" ? "mock-local" : null, interactionMode: agentId === "onepiece" ? "api" : "cli", configurationFingerprint: `mock-${agentId}-v1` },
    outcome: succeeded ? "succeeded" as const : "task_failed" as const,
    checks: [{ checkId: "deterministic-tests", passed: succeeded, summary: succeeded ? "42/42" : "41/42" }],
    metrics: [
      // `duration` in milliseconds, matching what the native engine emits
      // (evaluation_engine.rs:88). The mock used to call it `wall_time` in seconds, which the
      // results table -- which looks the metric up by name -- rendered as an empty cell forever.
      { name: "duration", value: 12_000 + index * 1_000, unit: "ms", quality: "reported" as const, source: "runtime" },
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

// Deliberately empty metrics: the native engine records none for an outcome that never reached
// verification (`aggregate_error`), and a mock that invented a duration here would teach the table
// to display something the desktop client never produces.
function webDispatchFailedAttempt(arenaId: string, task: EvaluationTask, agentId: string, index: number) {
  return {
    id: `${arenaId}-attempt-${index + 1}`, arenaId, canonicalRunId: `${arenaId}-run-${index + 1}`,
    taskId: task.id, taskVersion: task.version,
    agent: { agentId, providerId: "managed-cli", modelId: null, interactionMode: "cli", configurationFingerprint: `mock-${agentId}-v1` },
    outcome: "agent_failed" as const,
    checks: [{ ...WEB_DISPATCH_DIAGNOSTIC }],
    metrics: [],
    contextEvidenceManifestId: null, artifactIds: [],
    timeline: [
      { id: "prepare", kind: "lifecycle" as const, label: "Clean fixture prepared", status: "completed" },
      { id: "dispatch", kind: "lifecycle" as const, label: "Canonical evaluation attempt", status: "agent_failed" },
    ],
  };
}

// Mirrors `web-mission-control-client.ts`'s own `webMissionOverview`: the cursor is really just the
// offset re-exposed as an opaque string, so an invalid one is a real bug (never hand-typed by a
// reader, only ever round-tripped from a `nextCursor` this same client issued) and throws rather
// than silently resetting to page one, matching that same precedent's own choice.
function pageEvaluationArenas(query: EvaluationArenaQuery | undefined): EvaluationArenaPage {
  const limit = Math.max(1, Math.min(query?.limit ?? DEFAULT_EVALUATION_PAGE_LIMIT, MAX_EVALUATION_PAGE_LIMIT));
  const offset = query?.cursor ? Number(query.cursor) : 0;
  if (!Number.isSafeInteger(offset) || offset < 0) throw new Error("invalid evaluation cursor");
  return {
    items: structuredClone(webEvaluationArenas.slice(offset, offset + limit)),
    nextCursor: offset + limit < webEvaluationArenas.length ? String(offset + limit) : null,
  };
}

export const webEvaluationClient: EvaluationService = {
  async listEvaluationTasks() { return structuredClone(webEvaluationTasks); },
  async startEvaluation(input) {
    const task = webEvaluationTasks.find((item) => item.id === input.taskId && item.version === input.taskVersion);
    if (!task || input.agentIds.length === 0 || input.agentIds.length > 8) throw new Error("Invalid evaluation configuration");
    const id = `web-eval-${webEvaluationArenas.length + 1}`;
    const arena: EvaluationArena = { id, operationId: `${id}-operation`, taskId: task.id, taskVersion: task.version, rankingVersion: "deterministic-v2", attempts: input.agentIds.map((agentId, index) => webEvaluationAttempt(id, task, agentId, index)) };
    webEvaluationArenas = [arena, ...webEvaluationArenas]; return structuredClone(arena);
  },
  async listEvaluationArenas(query) { return pageEvaluationArenas(query); },
  async getEvaluationArena(arenaId) { const arena = webEvaluationArenas.find((item) => item.id === arenaId); if (!arena) throw new Error("Evaluation not found"); return structuredClone(arena); },
  async cancelEvaluation(this: AgentService, arenaId) { const arena = await this.getEvaluationArena(arenaId); const cancelled = { ...arena, attempts: arena.attempts.map((attempt) => ["queued", "running"].includes(attempt.outcome) ? { ...attempt, outcome: "cancelled" as const } : attempt) }; webEvaluationArenas = webEvaluationArenas.map((item) => item.id === arenaId ? cancelled : item); return structuredClone(cancelled); },
  async getEvaluationAttempt(attemptId) { for (const arena of webEvaluationArenas) { const attempt = arena.attempts.find((item) => item.id === attemptId); if (attempt) return structuredClone(attempt); } throw new Error("Evaluation attempt not found"); },
  async exportEvaluation(this: AgentService, arenaId) { return { schemaVersion: 1, arena: await this.getEvaluationArena(arenaId) }; },
};
