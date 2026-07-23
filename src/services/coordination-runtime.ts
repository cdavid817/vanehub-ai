import type {
  CoordinationAttempt,
  CoordinationFailureKind,
  CoordinationNodeInput,
  CoordinationNodeRun,
  CoordinationOutput,
  CoordinationRun,
  StartCoordinationInput,
} from "../types/coordination";

const outputByteLimit = 64 * 1024;
const contextByteLimit = 256 * 1024;
const stableIdPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export type CoordinationExecutionResult =
  | { status: "succeeded"; content: string }
  | { status: "failed"; kind: Exclude<CoordinationFailureKind, "cancelled">; error: string }
  | { status: "cancelled"; error: string };

export interface CoordinationExecutionRequest {
  runId: string;
  nodeId: string;
  agentId: string;
  attempt: number;
  candidateRole: "primary" | "fallback";
  instruction: string;
  prerequisiteContext: string;
  projectPath: string | null;
}

export type CoordinationExecutor = (
  request: CoordinationExecutionRequest,
) => CoordinationExecutionResult | Promise<CoordinationExecutionResult>;

export type CoordinationYield = () => Promise<void>;

export function validateCoordinationInput(input: StartCoordinationInput, knownAgentIds: ReadonlySet<string>): string[] {
  requiredText(input.name, "Coordination name");
  if (input.projectPath != null && input.projectPath !== "") {
    requiredText(input.projectPath, "Coordination project path");
  }
  if (input.nodes.length === 0) throw new Error("Coordination requires at least one node.");
  const nodes = new Map<string, CoordinationNodeInput>();
  for (const node of input.nodes) {
    stableId(node.id, "coordination node id");
    if (nodes.has(node.id)) throw new Error(`Duplicate coordination node id: ${node.id}`);
    requiredText(node.instruction, `Coordination node ${node.id} instruction`);
    const candidates = [node.primaryAgentId, ...node.fallbackAgentIds];
    candidates.forEach((agentId) => stableId(agentId, `Agent id for node ${node.id}`));
    if (new Set(candidates).size !== candidates.length) {
      throw new Error(`Coordination node ${node.id} repeats a primary or fallback Agent.`);
    }
    const unknownAgent = candidates.find((agentId) => !knownAgentIds.has(agentId));
    if (unknownAgent) throw new Error(`Unknown Agent id for node ${node.id}: ${unknownAgent}`);
    if (new Set(node.dependsOn).size !== node.dependsOn.length) {
      throw new Error(`Coordination node ${node.id} repeats a dependency.`);
    }
    node.dependsOn.forEach((dependencyId) => stableId(dependencyId, `Dependency id for node ${node.id}`));
    nodes.set(node.id, node);
  }

  const indegree = new Map<string, number>();
  const dependents = new Map<string, string[]>();
  for (const node of input.nodes) {
    indegree.set(node.id, node.dependsOn.length);
    for (const dependencyId of node.dependsOn) {
      if (dependencyId === node.id) throw new Error(`Coordination node ${node.id} cannot depend on itself.`);
      if (!nodes.has(dependencyId)) throw new Error(`Coordination node ${node.id} has missing dependency ${dependencyId}.`);
      dependents.set(dependencyId, [...(dependents.get(dependencyId) ?? []), node.id]);
    }
  }

  const ready = [...indegree.entries()].filter(([, value]) => value === 0).map(([id]) => id).sort();
  const order: string[] = [];
  while (ready.length > 0) {
    const id = ready.shift();
    if (!id) break;
    order.push(id);
    for (const dependentId of [...(dependents.get(id) ?? [])].sort()) {
      const next = (indegree.get(dependentId) ?? 0) - 1;
      indegree.set(dependentId, next);
      if (next === 0) {
        ready.push(dependentId);
        ready.sort();
      }
    }
  }
  if (order.length !== input.nodes.length) throw new Error("Coordination dependency graph contains a cycle.");
  return order;
}

export function createCoordinationRun(
  input: StartCoordinationInput,
  runId: string,
  operationId: string,
  timestamp: string,
  simulated: boolean,
): CoordinationRun {
  return {
    id: runId,
    operationId,
    name: input.name.trim(),
    projectPath: input.projectPath?.trim() || null,
    status: "queued",
    nodes: input.nodes.map((node) => ({
      ...node,
      fallbackAgentIds: [...node.fallbackAgentIds],
      dependsOn: [...node.dependsOn],
      instruction: node.instruction.trim(),
      status: node.dependsOn.length === 0 ? "queued" : "blocked",
      actualAgentId: null,
      output: null,
      attempts: [],
      error: null,
      startedAt: null,
      completedAt: null,
    })),
    simulated,
    cancelRequested: false,
    createdAt: timestamp,
    startedAt: null,
    updatedAt: timestamp,
    completedAt: null,
  };
}

export async function executeCoordinationRun(
  run: CoordinationRun,
  order: string[],
  executor: CoordinationExecutor,
  now: () => string,
  yieldTurn: CoordinationYield = yieldCoordinationTurn,
): Promise<CoordinationRun> {
  if (run.cancelRequested) return cancelPendingRun(run, now());
  run.status = "running";
  run.startedAt ??= now();
  run.updatedAt = run.startedAt;
  const nodes = new Map(run.nodes.map((node) => [node.id, node]));

  for (const nodeId of order) {
    await yieldTurn();
    const node = nodes.get(nodeId);
    if (!node || isTerminal(node.status)) continue;
    if (run.cancelRequested) {
      cancelRemaining(run, now());
      break;
    }
    const dependencies = node.dependsOn.map((id) => nodes.get(id)).filter((value): value is CoordinationNodeRun => Boolean(value));
    if (dependencies.some((dependency) => dependency.status !== "succeeded")) {
      node.status = "skipped";
      node.error = "A prerequisite node did not succeed.";
      node.completedAt = now();
      continue;
    }

    const context = assemblePrerequisiteContext(dependencies);
    if (new TextEncoder().encode(context).byteLength > contextByteLimit) {
      node.status = "failed";
      node.error = `Prerequisite context exceeds ${contextByteLimit} bytes.`;
      node.completedAt = now();
      continue;
    }

    node.status = "running";
    node.startedAt = now();
    const candidates = [node.primaryAgentId, ...node.fallbackAgentIds];
    for (const [index, agentId] of candidates.entries()) {
      if (run.cancelRequested) {
        node.status = "cancelled";
        node.error = "Coordination was cancelled.";
        node.completedAt = now();
        break;
      }
      const attempt = startAttempt(index, agentId, now());
      node.attempts.push(attempt);
      await yieldTurn();
      let result: CoordinationExecutionResult;
      if (run.cancelRequested) {
        result = cancelledExecutionResult();
      } else {
        result = await executor({
          runId: run.id,
          nodeId: node.id,
          agentId,
          attempt: attempt.attempt,
          candidateRole: attempt.candidateRole,
          instruction: node.instruction,
          prerequisiteContext: context,
          projectPath: run.projectPath,
        });
        await yieldTurn();
        if (run.cancelRequested) result = cancelledExecutionResult();
      }
      attempt.completedAt = now();
      if (result.status === "succeeded") {
        attempt.status = "succeeded";
        node.status = "succeeded";
        node.actualAgentId = agentId;
        node.output = boundedOutput(node.id, agentId, attempt.attempt, result.content);
        node.completedAt = attempt.completedAt;
        node.error = null;
        break;
      }
      attempt.status = result.status === "cancelled" ? "cancelled" : "failed";
      attempt.failureKind = result.status === "cancelled" ? "cancelled" : result.kind;
      attempt.error = result.error;
      node.error = result.error;
      if (result.status === "cancelled") {
        node.status = "cancelled";
        node.completedAt = attempt.completedAt;
        run.cancelRequested = true;
        break;
      }
      const canFailOver = result.kind === "retryable" && index < candidates.length - 1;
      if (!canFailOver) {
        node.status = "failed";
        node.completedAt = attempt.completedAt;
        break;
      }
    }
  }

  const timestamp = now();
  if (run.cancelRequested) {
    cancelRemaining(run, timestamp);
    run.status = "cancelled";
  } else {
    run.status = run.nodes.every((node) => node.status === "succeeded") ? "succeeded" : "failed";
  }
  run.updatedAt = timestamp;
  run.completedAt = timestamp;
  return run;
}

export function requestCoordinationCancellation(run: CoordinationRun, timestamp: string): CoordinationRun {
  if (["succeeded", "failed", "cancelled"].includes(run.status)) return run;
  run.cancelRequested = true;
  run.updatedAt = timestamp;
  if (run.status === "queued") cancelPendingRun(run, timestamp);
  return run;
}

function startAttempt(index: number, agentId: string, timestamp: string): CoordinationAttempt {
  return {
    attempt: index + 1,
    agentId,
    candidateRole: index === 0 ? "primary" : "fallback",
    status: "running",
    failureKind: null,
    error: null,
    startedAt: timestamp,
    completedAt: null,
  };
}

function boundedOutput(nodeId: string, agentId: string, attempt: number, content: string): CoordinationOutput {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(content);
  if (bytes.byteLength <= outputByteLimit) {
    return { sourceNodeId: nodeId, agentId, attempt, content, byteCount: bytes.byteLength, truncated: false };
  }
  let boundary = outputByteLimit;
  let bounded = "";
  while (boundary > 0) {
    try {
      bounded = new TextDecoder("utf-8", { fatal: true }).decode(bytes.slice(0, boundary));
      break;
    } catch {
      boundary -= 1;
    }
  }
  return { sourceNodeId: nodeId, agentId, attempt, content: bounded, byteCount: bytes.byteLength, truncated: true };
}

function assemblePrerequisiteContext(dependencies: CoordinationNodeRun[]): string {
  return dependencies.map((dependency) => {
    const output = dependency.output;
    if (!output) return "";
    return [
      `--- prerequisite node=${output.sourceNodeId} agent=${output.agentId} attempt=${output.attempt} ---`,
      output.content,
      "--- end prerequisite ---",
    ].join("\n");
  }).filter(Boolean).join("\n\n");
}

function cancelPendingRun(run: CoordinationRun, timestamp: string): CoordinationRun {
  cancelRemaining(run, timestamp);
  run.status = "cancelled";
  run.startedAt ??= timestamp;
  run.updatedAt = timestamp;
  run.completedAt = timestamp;
  return run;
}

function cancelRemaining(run: CoordinationRun, timestamp: string) {
  for (const node of run.nodes) {
    if (!isTerminal(node.status)) {
      node.status = node.status === "blocked" ? "skipped" : "cancelled";
      node.error = "Coordination was cancelled.";
      node.completedAt = timestamp;
    }
  }
}

function isTerminal(status: CoordinationNodeRun["status"]) {
  return ["succeeded", "failed", "skipped", "cancelled"].includes(status);
}

function stableId(value: string, label: string) {
  requiredText(value, label);
  if (!stableIdPattern.test(value)) throw new Error(`Invalid ${label}: ${value}`);
}

function requiredText(value: string, label: string) {
  if (!value.trim()) throw new Error(`${label} is required.`);
  if ([...value].some((character) => /\p{Cc}/u.test(character))) {
    throw new Error(`${label} contains control characters.`);
  }
}

function cancelledExecutionResult(): CoordinationExecutionResult {
  return { status: "cancelled", error: "Coordination was cancelled." };
}

function yieldCoordinationTurn() {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, 0);
  });
}
