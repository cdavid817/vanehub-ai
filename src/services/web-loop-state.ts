import { i18n } from "../i18n";
import type {
  LoopDefinition,
  LoopEvent,
  LoopEvidence,
  LoopIteration,
  LoopRun,
} from "../types/loop";
import { nowIso } from "./web-mock-clock";
import { deleteWebSessionMessages } from "./web-chat-state";
import { listWebSessions, replaceWebSessions } from "./web-session-state";

// Owned here and never exported. The scheduler, the client and the composition root all reach this
// state through the accessors below, so no importer can end up with a stale copy of it.
let loopDefinitions: LoopDefinition[] = [];
let loopRuns: LoopRun[] = [];
let nextLoopDefinitionId = 1;
let nextLoopRunId = 1;
let nextLoopEvidenceId = 1;
let webLoopPhaseDelayMs = 220;
const loopSubscribers = new Map<string, Set<(event: LoopEvent) => void>>();
const loopTimers = new Map<string, ReturnType<typeof setTimeout>>();
const loopRoleSessionIds = new Set<string>();

export function cloneLoopValue<T>(value: T): T {
  return structuredClone(value);
}

export function listWebLoopDefinitions(): LoopDefinition[] {
  return loopDefinitions;
}

export function replaceWebLoopDefinitions(next: LoopDefinition[]): void {
  loopDefinitions = next;
}

export function nextWebLoopDefinitionSequence(): number {
  const sequence = nextLoopDefinitionId;
  nextLoopDefinitionId += 1;
  return sequence;
}

export function listWebLoopRuns(): LoopRun[] {
  return loopRuns;
}

export function prependWebLoopRun(run: LoopRun): void {
  loopRuns = [run, ...loopRuns];
}

export function nextWebLoopRunSequence(): number {
  const sequence = nextLoopRunId;
  nextLoopRunId += 1;
  return sequence;
}

export function peekWebLoopRunSequence(): number {
  return nextLoopRunId;
}

export function getWebLoopPhaseDelayMs(): number {
  return webLoopPhaseDelayMs;
}

export function getWebLoopTimer(runId: string): ReturnType<typeof setTimeout> | undefined {
  return loopTimers.get(runId);
}

export function setWebLoopTimer(runId: string, timeoutId: ReturnType<typeof setTimeout>): void {
  loopTimers.set(runId, timeoutId);
}

export function clearWebLoopTimer(runId: string): void {
  loopTimers.delete(runId);
}

export function isWebLoopRoleSession(sessionId: string): boolean {
  return loopRoleSessionIds.has(sessionId);
}

export function addWebLoopRoleSession(sessionId: string): void {
  loopRoleSessionIds.add(sessionId);
}

/** A snapshot, not the live Set: the composition root only reads it to filter sessions. */
function snapshotWebLoopRoleSessionIds(): string[] {
  return [...loopRoleSessionIds];
}

export function subscribeWebLoopEvents(runId: string, handler: (event: LoopEvent) => void): () => void {
  const subscribers = loopSubscribers.get(runId) ?? new Set<(event: LoopEvent) => void>();
  subscribers.add(handler);
  loopSubscribers.set(runId, subscribers);
  return () => {
    subscribers.delete(handler);
    if (subscribers.size === 0) loopSubscribers.delete(runId);
  };
}

export function findLoopDefinition(definitionId: string): LoopDefinition {
  const definition = loopDefinitions.find((candidate) => candidate.id === definitionId);
  if (!definition) throw new Error(i18n.t("loops.web.error.definitionNotFound", { definitionId }));
  return definition;
}

export function findLoopRun(runId: string): LoopRun {
  const run = loopRuns.find((candidate) => candidate.id === runId);
  if (!run) throw new Error(i18n.t("loops.web.error.runNotFound", { runId }));
  return run;
}

export function emitLoopEvent(run: LoopRun, kind: LoopEvent["kind"] = "run-updated"): void {
  run.updatedAt = nowIso();
  const event: LoopEvent = { kind, run: cloneLoopValue(run) };
  loopSubscribers.get(run.id)?.forEach((handler) => handler(event));
}

export function addLoopEvidence(
  run: LoopRun,
  iteration: LoopIteration | null,
  input: Omit<LoopEvidence, "id" | "runId" | "iterationId" | "createdAt">,
): void {
  const evidence: LoopEvidence = {
    ...input,
    id: `web-loop-evidence-${nextLoopEvidenceId++}`,
    runId: run.id,
    iterationId: iteration?.id ?? null,
    createdAt: nowIso(),
  };
  if (iteration) iteration.evidence.push(evidence);
  emitLoopEvent(run, "evidence-added");
}

export function currentLoopIteration(run: LoopRun): LoopIteration {
  const iteration = run.iterations.at(-1);
  if (!iteration) throw new Error(i18n.t("loops.web.error.iterationNotFound", { runId: run.id }));
  return iteration;
}

export function createWebLoopIteration(runId: string, sequence: number, feedback: string | null): LoopIteration {
  return {
    id: `web-loop-iteration-${runId}-${sequence}`,
    runId,
    sequence,
    status: "running",
    workerSessionId: `web-loop-worker-${runId}-${sequence}`,
    verifierSessionId: null,
    workerSummary: null,
    verifierRecommendation: null,
    verifierFindings: [],
    decisionReason: null,
    diffFingerprint: null,
    checkFailureFingerprint: null,
    userFeedback: feedback,
    evidence: [],
    startedAt: nowIso(),
    completedAt: null,
  };
}

export function resetWebLoopsForTest(): void {
  loopTimers.forEach((timer) => clearTimeout(timer));
  loopTimers.clear();
  loopSubscribers.clear();
  const roleSessionIds = snapshotWebLoopRoleSessionIds();
  replaceWebSessions(listWebSessions().filter((session) => !roleSessionIds.includes(session.id)));
  roleSessionIds.forEach((sessionId) => deleteWebSessionMessages(sessionId));
  loopRoleSessionIds.clear();
  loopDefinitions = [];
  loopRuns = [];
  nextLoopDefinitionId = 1;
  nextLoopRunId = 1;
  nextLoopEvidenceId = 1;
}

export function simulateWebLoopRestartForTest(runId: string): LoopRun {
  const run = findLoopRun(runId);
  if (!["queued", "running", "awaiting-acceptance"].includes(run.status)) {
    throw new Error(i18n.t("loops.web.error.recoveryState"));
  }
  const timer = loopTimers.get(run.id);
  if (timer) clearTimeout(timer);
  loopTimers.delete(run.id);
  run.status = "paused";
  run.terminalReason = "recovery-required";
  run.pauseRequested = false;
  run.activeOperationId = null;
  emitLoopEvent(run);
  return cloneLoopValue(run);
}

export function setWebLoopPhaseDelayForTest(delayMs: number): void {
  webLoopPhaseDelayMs = Math.max(1, Math.min(delayMs, 10_000));
}
