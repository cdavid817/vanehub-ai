import { describe, expect, it } from "vitest";
import type { StartCoordinationInput } from "../types/coordination";
import {
  createCoordinationRun,
  executeCoordinationRun,
  requestCoordinationCancellation,
  type CoordinationExecutionResult,
  validateCoordinationInput,
} from "./coordination-runtime";

const agents = new Set(["claude-code", "codex-cli", "gemini-cli"]);

function input(nodes: StartCoordinationInput["nodes"]): StartCoordinationInput {
  return { name: "Review pipeline", projectPath: "D:\\project", nodes };
}

function node(
  id: string,
  primaryAgentId: string,
  dependsOn: string[] = [],
  fallbackAgentIds: string[] = [],
) {
  return { id, primaryAgentId, fallbackAgentIds, instruction: `Execute ${id}`, dependsOn };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolver) => {
    resolve = resolver;
  });
  return { promise, resolve };
}

describe("coordination runtime", () => {
  it("validates dependencies and returns deterministic topological order", () => {
    const plan = input([
      node("review", "claude-code", ["test", "implement"]),
      node("test", "gemini-cli", ["implement"]),
      node("docs", "claude-code"),
      node("implement", "codex-cli"),
    ]);

    expect(validateCoordinationInput(plan, agents)).toEqual(["docs", "implement", "test", "review"]);
    expect(() => validateCoordinationInput(input([
      node("alpha", "codex-cli", ["beta"]),
      node("beta", "claude-code", ["alpha"]),
    ]), agents)).toThrow("contains a cycle");
    expect(() => validateCoordinationInput(input([
      node("alpha", "Codex CLI"),
    ]), agents)).toThrow("Invalid Agent id");
  });

  it("passes prerequisite output with provenance into downstream context", async () => {
    const plan = input([
      node("implement", "codex-cli"),
      node("review", "claude-code", ["implement"]),
    ]);
    const order = validateCoordinationInput(plan, agents);
    const run = createCoordinationRun(plan, "run-1", "operation-1", "2026-07-23T00:00:00Z", true);
    const requests: string[] = [];

    await executeCoordinationRun(run, order, (request) => {
      requests.push(request.prerequisiteContext);
      return { status: "succeeded", content: `${request.nodeId} output` };
    }, () => "2026-07-23T00:01:00Z");

    expect(run.status).toBe("succeeded");
    expect(requests[1]).toContain("node=implement agent=codex-cli attempt=1");
    expect(requests[1]).toContain("implement output");
    expect(run.nodes[1].output?.sourceNodeId).toBe("review");
  });

  it("uses ordered fallbacks only for retryable failures", async () => {
    const plan = input([node("implement", "codex-cli", [], ["claude-code", "gemini-cli"])]);
    const run = createCoordinationRun(plan, "run-1", "operation-1", "2026-07-23T00:00:00Z", true);

    await executeCoordinationRun(run, validateCoordinationInput(plan, agents), (request) => {
      if (request.agentId === "codex-cli") {
        return { status: "failed", kind: "retryable", error: "process exited" };
      }
      return { status: "succeeded", content: "fallback result" };
    }, () => "2026-07-23T00:01:00Z");

    expect(run.nodes[0].attempts.map((attempt) => attempt.agentId)).toEqual(["codex-cli", "claude-code"]);
    expect(run.nodes[0].attempts[0].failureKind).toBe("retryable");
    expect(run.nodes[0].actualAgentId).toBe("claude-code");
    expect(run.status).toBe("succeeded");
  });

  it("does not mask non-retryable failure and skips dependents", async () => {
    const plan = input([
      node("implement", "codex-cli", [], ["claude-code"]),
      node("review", "gemini-cli", ["implement"]),
      node("docs", "claude-code"),
    ]);
    const run = createCoordinationRun(plan, "run-1", "operation-1", "2026-07-23T00:00:00Z", true);

    await executeCoordinationRun(run, validateCoordinationInput(plan, agents), (request) =>
      request.nodeId === "implement"
        ? { status: "failed", kind: "non-retryable", error: "policy rejected" }
        : { status: "succeeded", content: "independent output" },
    () => "2026-07-23T00:01:00Z");

    expect(run.nodes[0].attempts).toHaveLength(1);
    expect(run.nodes[1].status).toBe("skipped");
    expect(run.nodes[2].status).toBe("succeeded");
    expect(run.status).toBe("failed");
  });

  it("cancels a queued run idempotently", () => {
    const plan = input([node("implement", "codex-cli")]);
    const run = createCoordinationRun(plan, "run-1", "operation-1", "2026-07-23T00:00:00Z", true);

    requestCoordinationCancellation(run, "2026-07-23T00:00:01Z");
    requestCoordinationCancellation(run, "2026-07-23T00:00:02Z");

    expect(run.status).toBe("cancelled");
    expect(run.nodes[0].status).toBe("cancelled");
    expect(run.cancelRequested).toBe(true);
  });

  it("cancels an active attempt without starting fallback and settles remaining nodes", async () => {
    const plan = input([
      node("a-implement", "codex-cli", [], ["claude-code"]),
      node("b-review", "gemini-cli", ["a-implement"]),
      node("c-docs", "gemini-cli"),
    ]);
    const run = createCoordinationRun(plan, "run-1", "operation-1", "2026-07-23T00:00:00Z", true);
    const attemptStarted = deferred<void>();
    const attemptResult = deferred<CoordinationExecutionResult>();
    const requests: string[] = [];

    const execution = executeCoordinationRun(
      run,
      validateCoordinationInput(plan, agents),
      (request) => {
        requests.push(request.agentId);
        attemptStarted.resolve();
        return attemptResult.promise;
      },
      () => "2026-07-23T00:01:00Z",
      () => Promise.resolve(),
    );
    await attemptStarted.promise;
    expect(run.status).toBe("running");
    expect(run.nodes[0].attempts[0].status).toBe("running");

    requestCoordinationCancellation(run, "2026-07-23T00:01:01Z");
    requestCoordinationCancellation(run, "2026-07-23T00:01:02Z");
    attemptResult.resolve({ status: "failed", kind: "retryable", error: "late process failure" });
    await execution;

    expect(requests).toEqual(["codex-cli"]);
    expect(run.status).toBe("cancelled");
    expect(run.nodes[0].status).toBe("cancelled");
    expect(run.nodes[0].attempts).toHaveLength(1);
    expect(run.nodes[0].attempts[0]).toMatchObject({
      agentId: "codex-cli",
      status: "cancelled",
      failureKind: "cancelled",
    });
    expect(run.nodes[1].status).toBe("skipped");
    expect(run.nodes[2].status).toBe("cancelled");
  });

  it("truncates multibyte output at a valid UTF-8 boundary", async () => {
    const plan = input([node("implement", "codex-cli")]);
    const run = createCoordinationRun(plan, "run-1", "operation-1", "2026-07-23T00:00:00Z", true);

    await executeCoordinationRun(
      run,
      validateCoordinationInput(plan, agents),
      () => ({ status: "succeeded", content: "界".repeat(64 * 1024) }),
      () => "2026-07-23T00:01:00Z",
    );

    expect(run.nodes[0].output?.truncated).toBe(true);
    expect(new TextEncoder().encode(run.nodes[0].output?.content).byteLength).toBeLessThanOrEqual(64 * 1024);
    expect(run.nodes[0].output?.content.includes("\uFFFD")).toBe(false);
  });
});
