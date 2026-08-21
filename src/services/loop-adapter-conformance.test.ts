import { beforeEach, describe, expect, it } from "vitest";
import { webAgentClient, resetWebLoopsForTest } from "./web-agent-client";
import type { LoopWorkbenchService } from "./loop-service";
import { webLoopClient } from "./web-loop-client";

const webLoopAdapter: LoopWorkbenchService = { ...webAgentClient, ...webLoopClient };

const contractMethods = [
  "listLoopProjectChoices", "listLoopBranches", "checkLoopReadiness",
  "listLoopDefinitions", "createLoopDefinition", "updateLoopDefinition", "deleteLoopDefinition",
  "listLoopRuns", "getLoopRun", "startLoop", "pauseLoop", "resumeLoop", "cancelLoop",
  "acceptLoop", "continueLoop", "rejectLoop", "subscribeLoopEvents",
] as const satisfies readonly (keyof LoopWorkbenchService)[];

describe("Loop adapter conformance", () => {
  beforeEach(() => resetWebLoopsForTest());

  it("exposes the complete Loop service contract from the Web adapter", () => {
    for (const method of contractMethods) expect(webLoopAdapter[method]).toEqual(expect.any(Function));
  });

  it("keeps Web discovery and readiness explicitly simulated and non-launching", async () => {
    const definition = await webAgentClient.createLoopDefinition({
      name: "Conformance Loop", enabled: true, projectPath: "D:/example-workspace", baseBranch: "main",
      goal: "Verify parity", acceptanceCriteria: ["Checks pass"], allowedPaths: ["src"], protectedPaths: [".git"],
      workerAgentId: "codex-cli", verifierAgentId: "claude-code",
      verificationCommands: [{ id: "tests", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }],
      limits: { maxIterations: 3, stepTimeoutSeconds: 60, totalTimeoutSeconds: 600, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    });

    const projects = await webLoopAdapter.listLoopProjectChoices();
    const branches = await webLoopAdapter.listLoopBranches(definition.projectPath);
    const readiness = await webLoopAdapter.checkLoopReadiness(definition.id);

    expect(projects.every((choice) => choice.simulated)).toBe(true);
    expect(branches.every((choice) => choice.simulated)).toBe(true);
    expect(readiness).toMatchObject({ definitionId: definition.id, ready: true, simulated: true });
    expect(await webAgentClient.listLoopRuns(definition.id)).toEqual([]);
    expect(readiness.checks.find((check) => check.code === "worker-eligible")?.status).toBe("passed");
    expect(readiness.checks.find((check) => check.code === "verifier-eligible")?.status).toBe("passed");
  });

  it("retains unavailable saved project and branch values with explicit labels", async () => {
    const definition = await webAgentClient.createLoopDefinition({
      name: "Unavailable Loop", enabled: true, projectPath: "D:/missing", baseBranch: "deleted-branch",
      goal: "Keep selections", acceptanceCriteria: ["Visible"], allowedPaths: ["src"], protectedPaths: [".git"],
      workerAgentId: "codex-cli", verifierAgentId: "claude-code",
      verificationCommands: [{ id: "tests", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }],
      limits: { maxIterations: 3, stepTimeoutSeconds: 60, totalTimeoutSeconds: 600, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    });
    const projects = await webLoopAdapter.listLoopProjectChoices();
    const branches = await webLoopAdapter.listLoopBranches(definition.projectPath);
    const readiness = await webLoopAdapter.checkLoopReadiness(definition.id);

    expect(projects).toContainEqual(expect.objectContaining({ path: "D:/missing", displayName: "missing", available: false, simulated: true }));
    expect(branches).toContainEqual(expect.objectContaining({ name: "deleted-branch", available: false, simulated: true }));
    expect(readiness.ready).toBe(false);
  });
});
