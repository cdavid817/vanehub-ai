import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LoopRun, SaveLoopDefinitionInput } from "../types/loop";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";

const input = {
  name: "Loop",
  enabled: true,
  projectPath: "D:/project",
  baseBranch: "main",
  goal: "Ship change",
  acceptanceCriteria: ["Tests pass"],
  allowedPaths: ["src"],
  protectedPaths: [".git"],
  workerAgentId: "codex-cli",
  verifierAgentId: "claude-code",
  verificationCommands: [],
  limits: {
    maxIterations: 3,
    stepTimeoutSeconds: 60,
    totalTimeoutSeconds: 600,
    maxConsecutiveRuntimeErrors: 2,
    maxConsecutiveNoProgress: 2,
  },
} satisfies SaveLoopDefinitionInput;

describe("Tauri Loop adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ id: "run-1", updatedAt: "now" } as LoopRun);
  });

  afterEach(() => vi.useRealTimers());

  it("maps every Loop management and control call to its thin command", async () => {
    await tauriAgentClient.listLoopDefinitions();
    await tauriAgentClient.createLoopDefinition(input);
    await tauriAgentClient.updateLoopDefinition("definition-1", input);
    await tauriAgentClient.deleteLoopDefinition("definition-1");
    await tauriAgentClient.listLoopRuns();
    await tauriAgentClient.listLoopRuns("definition-1");
    await tauriAgentClient.getLoopRun("run-1");
    await tauriAgentClient.startLoop("definition-1");
    await tauriAgentClient.pauseLoop("run-1");
    await tauriAgentClient.resumeLoop("run-1");
    await tauriAgentClient.cancelLoop("run-1");
    await tauriAgentClient.acceptLoop("run-1");
    await tauriAgentClient.continueLoop({ runId: "run-1", feedback: "Revise tests" });
    await tauriAgentClient.rejectLoop("run-1");

    expect(invokeMock.mock.calls).toEqual([
      ["list_loop_definitions"],
      ["create_loop_definition", { input }],
      ["update_loop_definition", { definitionId: "definition-1", input }],
      ["delete_loop_definition", { definitionId: "definition-1" }],
      ["list_loop_runs", { definitionId: null }],
      ["list_loop_runs", { definitionId: "definition-1" }],
      ["get_loop_run", { runId: "run-1" }],
      ["start_loop", { definitionId: "definition-1" }],
      ["pause_loop", { runId: "run-1" }],
      ["resume_loop", { runId: "run-1" }],
      ["cancel_loop", { runId: "run-1" }],
      ["accept_loop", { runId: "run-1" }],
      ["continue_loop", { input: { runId: "run-1", feedback: "Revise tests" } }],
      ["reject_loop", { runId: "run-1" }],
    ]);
  });

  it("adapts native run snapshots into Loop events and stops polling on unsubscribe", async () => {
    vi.useFakeTimers();
    const initial = { id: "run-1", status: "running", updatedAt: "first" } as LoopRun;
    const updated = { ...initial, status: "paused" as const, updatedAt: "second" };
    invokeMock.mockReset();
    invokeMock.mockResolvedValueOnce(initial).mockResolvedValue(updated);
    const handler = vi.fn();

    const unsubscribe = await tauriAgentClient.subscribeLoopEvents("run-1", handler);
    await vi.advanceTimersByTimeAsync(1_000);

    expect(invokeMock.mock.calls).toEqual([
      ["get_loop_run", { runId: "run-1" }],
      ["get_loop_run", { runId: "run-1" }],
    ]);
    expect(handler).toHaveBeenCalledWith({ kind: "run-updated", run: updated });

    unsubscribe();
    await vi.advanceTimersByTimeAsync(2_000);
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });
});
