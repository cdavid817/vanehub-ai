import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriCliEnvironmentClient } from "./tauri-cli-environment-client";
import { webCliEnvironmentClient } from "./web-cli-environment-client";
import { webOperationClient } from "./web-operation-client";
import {
  WEB_CLI_FIXED_PLAN_IDS,
  WEB_CLI_OUTCOME_TARGETS,
} from "./web-cli-environment-fixtures";
import type { OperationStatus } from "../types/operation";
import {
  CLI_BULK_SKIP_REASONS,
  CLI_MUTATION_OUTCOMES,
  type CliBulkExecutionResult,
} from "../types/cli-environment";

// The lifecycle both runtimes must agree on. Adding a status here without adding it to the Rust
// `OperationStatus` enum is exactly the drift this file exists to catch.
const LIFECYCLE: OperationStatus[] = ["queued", "running", "succeeded", "failed", "cancelled"];

async function waitForTerminal(operationId: string) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    const task = await webOperationClient.getOperationStatus(operationId);
    if (["succeeded", "failed", "cancelled"].includes(task.status)) return task;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error(`operation ${operationId} never reached a terminal status`);
}

/** Drives one mutation end to end through the Web runtime and returns its terminal result. */
async function webMutation(targetVersion: string) {
  const planning = await webCliEnvironmentClient.prepareCliAction({
    agentId: "claude-code",
    action: null,
    sourceId: "npm",
    targetVersion,
    channel: null,
  });
  const planned = await waitForTerminal(planning.id);
  const planId = (planned.result as { planId: string }).planId;
  const execution = await webCliEnvironmentClient.executeCliAction({ planId, expectedRevision: 1 });
  return waitForTerminal(execution.id);
}

describe("CLI environment adapter conformance across runtimes", () => {
  beforeEach(() => invokeMock.mockReset());

  it("relays every method to exactly one command with the caller's own arguments", async () => {
    invokeMock.mockResolvedValue({ operationId: "op-1" });
    invokeMock.mockResolvedValueOnce([]);
    await tauriCliEnvironmentClient.listCliEnvironments();
    await tauriCliEnvironmentClient.refreshCliEnvironments(["claude-code"], true);
    await tauriCliEnvironmentClient.prepareCliAction({
      agentId: "claude-code",
      action: null,
      sourceId: "npm",
      targetVersion: "1.1.0",
      channel: "stable",
    });
    invokeMock.mockResolvedValueOnce({ id: "plan-1" });
    await tauriCliEnvironmentClient.getCliActionPlan("plan-1");
    await tauriCliEnvironmentClient.executeCliAction({ planId: "plan-1", expectedRevision: 3 });
    await tauriCliEnvironmentClient.prepareCliBulkUpgrade(["claude-code"]);
    invokeMock.mockResolvedValueOnce({ id: "bulk-1" });
    await tauriCliEnvironmentClient.getCliBulkActionPlan("bulk-1");
    await tauriCliEnvironmentClient.executeCliBulkAction({ planId: "bulk-1", expectedRevision: 1 });
    await tauriCliEnvironmentClient.runCliDoctor("claude-code");

    expect(invokeMock.mock.calls.map(([command]) => command)).toEqual([
      "list_cli_environments",
      "refresh_cli_environment",
      "prepare_cli_action",
      "get_cli_action_plan",
      "execute_cli_action",
      "prepare_cli_bulk_action",
      "get_cli_bulk_action_plan",
      "execute_cli_bulk_action",
      "run_cli_doctor",
    ]);
  });

  it("sends the chosen source, channel, and target through untouched", async () => {
    invokeMock.mockResolvedValue({ operationId: "op-1" });
    await tauriCliEnvironmentClient.prepareCliAction({
      agentId: "claude-code",
      action: null,
      sourceId: "npm",
      targetVersion: "1.1.0",
      channel: "stable",
    });

    // The older version the user picked, not the newest one. No substitution, and no action:
    // the backend derives the direction so this side never compares two versions.
    expect(invokeMock).toHaveBeenCalledWith("prepare_cli_action", {
      agentId: "claude-code",
      action: null,
      sourceId: "npm",
      targetVersion: "1.1.0",
      channel: "stable",
    });
  });

  it("sends only a plan id and revision to execute", async () => {
    invokeMock.mockResolvedValue({ operationId: "op-1" });
    await tauriCliEnvironmentClient.executeCliAction({ planId: "plan-9", expectedRevision: 4 });

    const [, payload] = invokeMock.mock.calls[0];
    // Nothing else crosses, so nothing else can be substituted between review and execution.
    expect(Object.keys(payload as object).sort()).toEqual(["expectedRevision", "planId"]);
  });

  it("returns a watchable operation from every variable-duration method", async () => {
    invokeMock.mockResolvedValue({ operationId: "op-7" });
    for (const started of [
      await tauriCliEnvironmentClient.refreshCliEnvironments([], false),
      await tauriCliEnvironmentClient.prepareCliBulkUpgrade([]),
      await tauriCliEnvironmentClient.runCliDoctor("claude-code"),
    ]) {
      expect(started.id).toBe("op-7");
      expect(started.kind).toBe("cli");
      // Queued, not running: the backend accepted the work and has started nothing observable.
      expect(started.status).toBe("queued");
      expect(LIFECYCLE).toContain(started.status);
    }
  });

  it("does not invent progress metadata the backend did not send", async () => {
    invokeMock.mockResolvedValue({ operationId: "op-8" });
    const started = await tauriCliEnvironmentClient.runCliDoctor("claude-code");

    expect(started.phase ?? null).toBeNull();
    expect(started.cancellable ?? null).toBeNull();
    expect(started.completedUnits ?? null).toBeNull();
  });

  it("serves deterministic Web snapshots that never claim to have read the host", async () => {
    const snapshots = await webCliEnvironmentClient.listCliEnvironments();

    expect(snapshots.length).toBeGreaterThan(0);
    for (const snapshot of snapshots) {
      expect(snapshot.displayName).not.toBe("");
      for (const installation of snapshot.installations) {
        // Obvious placeholders. A realistic home directory would read as a real finding on a page
        // that cannot have looked at one.
        expect(installation.executablePath.startsWith("/mock/")).toBe(true);
      }
    }
    // Every distinct case the UI has to render is present.
    expect(snapshots.some((snapshot) => snapshot.conflicts.length > 0)).toBe(true);
    expect(snapshots.some((snapshot) => snapshot.installations.length === 0)).toBe(true);
    expect(snapshots.some((snapshot) => snapshot.update === "not-applicable")).toBe(true);
  });

  it("reaches every terminal mutation outcome deterministically on the Web runtime", async () => {
    const cases: Array<[string, string, OperationStatus]> = [
      [WEB_CLI_OUTCOME_TARGETS.verified, "verified", "succeeded"],
      [WEB_CLI_OUTCOME_TARGETS.appliedUnverified, "applied-unverified", "succeeded"],
      [WEB_CLI_OUTCOME_TARGETS.changedButFailed, "changed-but-failed", "failed"],
      [WEB_CLI_OUTCOME_TARGETS.noChangeFailed, "no-change-failed", "failed"],
      [WEB_CLI_OUTCOME_TARGETS.cancelled, "cancelled", "failed"],
    ];

    for (const [target, outcome, status] of cases) {
      const terminal = await webMutation(target);
      expect(terminal.status).toBe(status);
      expect((terminal.result as { outcome: string }).outcome).toBe(outcome);
      // The version the plan aimed at survives to the result, whatever the outcome was.
      expect((terminal.result as { targetVersion: string }).targetVersion).toBe(target);
    }
  });

  it("refuses an expired, consumed, or superseded plan before anything runs", async () => {
    await expect(
      webCliEnvironmentClient.executeCliAction({ planId: WEB_CLI_FIXED_PLAN_IDS.expired, expectedRevision: 1 }),
    ).rejects.toThrow("plan-expired");
    await expect(
      webCliEnvironmentClient.executeCliAction({ planId: WEB_CLI_FIXED_PLAN_IDS.consumed, expectedRevision: 1 }),
    ).rejects.toThrow("plan-consumed");
    // The environment moved, so the revision the caller saw is no longer the current one.
    await expect(
      webCliEnvironmentClient.executeCliAction({ planId: WEB_CLI_FIXED_PLAN_IDS.stale, expectedRevision: 1 }),
    ).rejects.toThrow("plan-revision-mismatch");
  });

  it("reports bulk eligibility and skips with a reason on the Web runtime", async () => {
    const started = await webCliEnvironmentClient.prepareCliBulkUpgrade([
      "claude-code",
      "gemini-cli",
      "opencode",
    ]);
    const terminal = await waitForTerminal(started.id);
    const result = terminal.result as { items: number; skipped: Array<{ agentId: string; reason: string }> };

    expect(result.items).toBe(1);
    // A silently shorter item list would read as "everything else is already up to date".
    expect(result.skipped.map((skip) => skip.agentId).sort()).toEqual(["gemini-cli", "opencode"]);
    expect(result.skipped.find((skip) => skip.agentId === "opencode")?.reason).toBe("installation-conflict");
  });

  it("reports a typed per-item result for a bulk execution", async () => {
    const started = await webCliEnvironmentClient.executeCliBulkAction({
      planId: "web-bulk-plan",
      expectedRevision: 1,
    });
    const terminal = await waitForTerminal(started.id);
    const { items } = terminal.result as unknown as CliBulkExecutionResult;

    // A batch that half-succeeded is not a batch that succeeded.
    expect(items.map((item) => item.status)).toEqual(["completed", "completed", "skipped"]);
    expect(items.filter((item) => item.status === "completed").map((item) => item.outcome))
      .toEqual(["verified", "no-change-failed"]);
    for (const item of items) {
      // Exactly one arm is populated; a reader never has to tell "absent" from "not applicable".
      if (item.status === "completed") {
        expect(CLI_MUTATION_OUTCOMES).toContain(item.outcome);
        expect(item.reason).toBeNull();
      } else {
        expect(CLI_BULK_SKIP_REASONS).toContain(item.reason);
        expect(item.outcome).toBeNull();
      }
    }
    // The placeholder said a process started and nothing about whether the machine changed.
    expect(JSON.stringify(items)).not.toContain("\"ran\"");
  });

  it("supports Web cancellation as a distinct terminal status with no invented result", async () => {
    const started = await webCliEnvironmentClient.runCliDoctor("claude-code");
    const cancelled = await webOperationClient.cancelOperation(started.id);

    expect(cancelled.status).toBe("cancelled");
    expect(cancelled.kind).toBe("cli");
    // Cancelling is not a rollback claim: nothing is fabricated for work that never finished.
    expect(cancelled.result).toBeNull();
  });

  it("answers a doctor probe with unknown rather than a fabricated verdict", async () => {
    const started = await webCliEnvironmentClient.runCliDoctor("claude-code");
    const terminal = await waitForTerminal(started.id);

    // A browser cannot run the tool's own diagnostics; `unknown` is the honest answer.
    expect((terminal.result as { doctor: string }).doctor).toBe("unknown");
  });
});
