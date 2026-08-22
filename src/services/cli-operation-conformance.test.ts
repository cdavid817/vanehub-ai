import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriAgentClient } from "./tauri-agent-client";
import { webCliToolClient } from "./web-cli-tool-client";
import { webOperationClient } from "./web-operation-client";
import type { OperationStatus, OperationTask } from "../types/operation";

// The lifecycle both runtimes must agree on. Adding a status here without adding it to the Rust
// `OperationStatus` enum is exactly the drift this file exists to catch.
const LIFECYCLE: OperationStatus[] = ["queued", "running", "succeeded", "failed", "cancelled"];

function nativeOperation(status: OperationStatus): OperationTask {
  return {
    id: `op-${status}`,
    kind: "cli",
    status,
    relatedEntityId: "claude-code",
    message: null,
    logs: [],
    result: null,
    error: status === "failed" ? "npm exited with code 1" : null,
    createdAt: "1",
    updatedAt: "2",
  };
}

async function waitForStatus(operationId: string, predicate: (task: OperationTask) => boolean) {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    const task = await webOperationClient.getOperationStatus(operationId);
    if (predicate(task)) return task;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error(`operation ${operationId} never reached the expected status`);
}

describe("CLI operation contract conformance across runtimes", () => {
  beforeEach(() => invokeMock.mockReset());

  it("marks Web/mock CLI work with the cli operation kind", async () => {
    const refresh = await webCliToolClient.refreshCliDetections("claude-code");
    expect(refresh.kind).toBe("cli");
    expect(refresh.relatedEntityId).toBe("claude-code");

    const install = await webCliToolClient.installCliVersion({
      agentId: "claude-code",
      targetVersion: "1.2.3",
    });
    expect(install.kind).toBe("cli");

    const bulk = await webCliToolClient.upgradeAllCliVersions();
    expect(bulk.kind).toBe("cli");
  });

  it("moves a Web/mock CLI operation through queued, running, and a terminal status", async () => {
    const started = await webCliToolClient.refreshCliDetections();
    expect(started.status).toBe("queued");

    const running = await waitForStatus(started.id, (task) => task.status !== "queued");
    // Deterministic simulation, not a host inspection: the fixture never claims to have read PATH.
    expect(["running", "failed", "succeeded"]).toContain(running.status);

    const terminal = await waitForStatus(started.id, (task) =>
      ["succeeded", "failed", "cancelled"].includes(task.status),
    );
    expect(LIFECYCLE).toContain(terminal.status);
    expect(terminal.kind).toBe("cli");
  });

  it("supports Web/mock CLI cancellation as a distinct terminal status", async () => {
    const started = await webCliToolClient.installCliVersion({
      agentId: "codex-cli",
      targetVersion: "1.0.0",
    });

    const cancelled = await webOperationClient.cancelOperation(started.id);

    expect(cancelled.status).toBe("cancelled");
    expect(cancelled.kind).toBe("cli");
    // Cancelling is not a rollback claim: no result is fabricated for work that never ran.
    expect(cancelled.result).toBeNull();
  });

  it("passes every native CLI lifecycle status through the Tauri adapter unchanged", async () => {
    for (const status of LIFECYCLE) {
      invokeMock.mockResolvedValueOnce(nativeOperation(status));
      const task = await tauriAgentClient.refreshCliDetections("claude-code");
      expect(task.status).toBe(status);
      expect(task.kind).toBe("cli");
    }
    expect(invokeMock).toHaveBeenCalledTimes(LIFECYCLE.length);
  });

  it("keeps optional progress metadata optional on both runtimes", async () => {
    // Native payload without progress: the adapter must not invent phase or cancellability.
    invokeMock.mockResolvedValueOnce(nativeOperation("running"));
    const withoutProgress = await tauriAgentClient.refreshCliDetections();
    expect(withoutProgress.phase ?? null).toBeNull();
    expect(withoutProgress.completedUnits ?? null).toBeNull();
    expect(withoutProgress.totalUnits ?? null).toBeNull();
    expect(withoutProgress.cancellable ?? null).toBeNull();

    // Native payload with progress: the adapter must pass it through verbatim.
    invokeMock.mockResolvedValueOnce({
      ...nativeOperation("running"),
      phase: "querying-catalog",
      completedUnits: 1,
      totalUnits: 3,
      cancellable: true,
    });
    const withProgress = await tauriAgentClient.refreshCliDetections();
    expect(withProgress.phase).toBe("querying-catalog");
    expect(withProgress.completedUnits).toBe(1);
    expect(withProgress.totalUnits).toBe(3);
    expect(withProgress.cancellable).toBe(true);

    const webTask = await webCliToolClient.refreshCliDetections();
    expect(webTask.phase ?? null).toBeNull();
    expect(webTask.cancellable ?? null).toBeNull();
  });
});
