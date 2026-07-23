import { afterEach, describe, expect, it } from "vitest";
import type { CoordinationRun, StartCoordinationInput } from "../types/coordination";
import { resetWebLoopsForTest, webAgentClient } from "./web-agent-client";

afterEach(() => {
  resetWebLoopsForTest();
});

async function waitForRun(
  runId: string,
  predicate: (run: CoordinationRun) => boolean,
): Promise<CoordinationRun> {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const run = await webAgentClient.getCoordinationRun(runId);
    if (predicate(run)) return run;
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 5);
    });
  }
  throw new Error(`Timed out waiting for Web coordination run ${runId}.`);
}

describe("Web coordination scheduling", () => {
  it("cancels a running attempt without starting fallback and settles remaining nodes", async () => {
    const input: StartCoordinationInput = {
      name: "Active cancellation",
      projectPath: null,
      nodes: [
        {
          id: "a-implement",
          primaryAgentId: "codex-cli",
          fallbackAgentIds: ["claude-code"],
          instruction: "Implement the change",
          dependsOn: [],
        },
        {
          id: "b-review",
          primaryAgentId: "gemini-cli",
          fallbackAgentIds: [],
          instruction: "Review the implementation",
          dependsOn: ["a-implement"],
        },
        {
          id: "c-docs",
          primaryAgentId: "opencode",
          fallbackAgentIds: [],
          instruction: "Document the change",
          dependsOn: [],
        },
      ],
    };

    const started = await webAgentClient.startCoordination(input);
    const running = await waitForRun(
      started.runId,
      (run) => run.status === "running"
        && run.nodes[0].status === "running"
        && run.nodes[0].attempts[0]?.status === "running",
    );
    expect(running.nodes[0].attempts).toHaveLength(1);

    const cancelling = await webAgentClient.cancelCoordinationRun(started.runId);
    expect(cancelling).toMatchObject({ status: "running", cancelRequested: true });
    await webAgentClient.cancelCoordinationRun(started.runId);

    const cancelled = await waitForRun(started.runId, (run) => run.status === "cancelled");
    const activeNode = cancelled.nodes.find((node) => node.id === "a-implement");
    const dependentNode = cancelled.nodes.find((node) => node.id === "b-review");
    const independentNode = cancelled.nodes.find((node) => node.id === "c-docs");

    expect(activeNode).toMatchObject({
      status: "cancelled",
      actualAgentId: null,
      attempts: [{
        agentId: "codex-cli",
        candidateRole: "primary",
        status: "cancelled",
        failureKind: "cancelled",
      }],
    });
    expect(dependentNode?.status).toBe("skipped");
    expect(independentNode?.status).toBe("cancelled");
    expect(activeNode?.attempts.some((attempt) => attempt.candidateRole === "fallback")).toBe(false);
  });
});
