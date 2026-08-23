import type { CliToolService } from "./cli-service";
import { createWebMockOperation } from "./web-operation-client";
import { nowIso } from "./web-mock-clock";
import {
  WEB_CLI_FIXED_PLAN_IDS,
  WEB_CLI_OUTCOME_TARGETS,
  webCliActionPlan,
  webCliEnvironmentSnapshots,
} from "./web-cli-environment-fixtures";
import type {
  CliActionPlan,
  CliBulkActionPlan,
  ExecuteCliActionInput,
  PrepareCliActionInput,
} from "../types/cli-environment-snapshot";
import type { OperationTask } from "../types/operation";

/**
 * The Web/mock half of the source-aware CLI surface.
 *
 * Mirrors `tauri-cli-environment-client.ts` method for method. Every result is invented and
 * deterministic: no process, no package manager, no network, no credential store, no host PATH,
 * and no feature-local log.
 *
 * The point of the determinism is that each terminal outcome the desktop runtime can produce is
 * reachable here on purpose, so a UI can be exercised against `applied-unverified` and
 * `changed-but-failed` without a machine that happens to be in that state.
 */

/** Which outcome a chosen target version drives. Unlisted targets verify. */
function outcomeFor(targetVersion: string | null): {
  status: "succeeded" | "failed";
  outcome: string;
  error: string | null;
} {
  switch (targetVersion) {
    case WEB_CLI_OUTCOME_TARGETS.appliedUnverified:
      // The command succeeded; verification could not confirm it. Not a failure.
      return { status: "succeeded", outcome: "applied-unverified", error: null };
    case WEB_CLI_OUTCOME_TARGETS.changedButFailed:
      return { status: "failed", outcome: "changed-but-failed", error: "cli.mutation.changed-but-failed" };
    case WEB_CLI_OUTCOME_TARGETS.noChangeFailed:
      return { status: "failed", outcome: "no-change-failed", error: "cli.mutation.no-change-failed" };
    case WEB_CLI_OUTCOME_TARGETS.cancelled:
      return { status: "failed", outcome: "cancelled", error: "cli.mutation.cancelled" };
    default:
      return { status: "succeeded", outcome: "verified", error: null };
  }
}

function mutationResult(
  operationId: string,
  input: PrepareCliActionInput,
  outcome: ReturnType<typeof outcomeFor>,
): Record<string, unknown> {
  const observed = outcome.outcome === "verified" ? input.targetVersion : "1.2.0";
  return {
    operationId,
    agentId: input.agentId,
    sourceId: input.sourceId,
    action: input.action,
    targetVersion: input.targetVersion,
    observedVersion: observed,
    phase: "completed",
    termination: outcome.outcome === "cancelled" ? "cancelled" : "exited",
    exitCode: outcome.outcome === "cancelled" ? null : 0,
    elapsedMs: 900,
    outcome: outcome.outcome,
    warnings: outcome.outcome === "applied-unverified" ? ["target-version-not-observed"] : [],
    outputTruncated: false,
    warning: outcome.outcome !== "verified",
  };
}

/** Plans the mock always answers for, one per refusal the desktop runtime can produce. */
function fixedPlan(planId: string): CliActionPlan | null {
  switch (planId) {
    case WEB_CLI_FIXED_PLAN_IDS.expired:
      return webCliActionPlan({ id: planId, state: "expired" });
    case WEB_CLI_FIXED_PLAN_IDS.consumed:
      return webCliActionPlan({ id: planId, state: "completed" });
    case WEB_CLI_FIXED_PLAN_IDS.stale:
      // A revision the caller cannot match: the environment moved after the plan was built.
      return webCliActionPlan({ id: planId, revision: 2 });
    case WEB_CLI_FIXED_PLAN_IDS.draft:
      return webCliActionPlan({ id: planId });
    default:
      return null;
  }
}

let preparedPlan: CliActionPlan | null = null;

export const webCliEnvironmentClient: CliToolService = {
  async listCliEnvironments() {
    return webCliEnvironmentSnapshots();
  },

  async refreshCliEnvironments(agentIds: string[], forceCatalog: boolean): Promise<OperationTask> {
    const requested = agentIds.length > 0 ? agentIds : webCliEnvironmentSnapshots().map((item) => item.agentId);
    return createWebMockOperation({
      id: `web-cli-refresh-${nowIso()}`,
      kind: "cli",
      relatedEntityId: agentIds.length === 1 ? agentIds[0] : null,
      message: "cli.operation.refresh",
      terminalStatus: "succeeded",
      error: null,
      result: { agentIds: requested, forceCatalog },
    });
  },

  async prepareCliAction(input: PrepareCliActionInput): Promise<OperationTask> {
    // The chosen source, channel, and target land on the plan unchanged. Substituting any of them
    // here would reproduce, in the mock, the defect the action plan exists to prevent.
    preparedPlan = webCliActionPlan({
      id: `web-plan-${input.agentId}-${nowIso()}`,
      agentId: input.agentId,
      // No action named: the desktop backend derives the direction, and the mock says `upgrade` so
      // a plan always has one rather than leaving the field empty and calling that a decision.
      action: input.action ?? "upgrade",
      sourceId: input.sourceId,
      targetVersion: input.targetVersion,
      channel: input.channel,
    });
    return createWebMockOperation({
      id: `web-cli-plan-${input.agentId}-${nowIso()}`,
      kind: "cli",
      relatedEntityId: input.agentId,
      message: "cli.operation.plan",
      terminalStatus: "succeeded",
      error: null,
      result: { planId: preparedPlan.id, revision: preparedPlan.revision },
    });
  },

  async getCliActionPlan(planId: string): Promise<CliActionPlan> {
    const fixed = fixedPlan(planId);
    if (fixed) return fixed;
    if (preparedPlan?.id === planId) return preparedPlan;
    throw new Error(`plan not found: ${planId}`);
  },

  async executeCliAction(input: ExecuteCliActionInput): Promise<OperationTask> {
    const plan = await this.getCliActionPlan(input.planId);
    if (plan.revision !== input.expectedRevision) {
      // Refused before anything runs, exactly as the desktop repository refuses it.
      throw new Error("cli.error.plan-revision-mismatch");
    }
    if (plan.state !== "draft") {
      throw new Error(plan.state === "expired" ? "cli.error.plan-expired" : "cli.error.plan-consumed");
    }
    const outcome = outcomeFor(plan.targetVersion);
    const operationId = `web-cli-execute-${plan.id}`;
    return createWebMockOperation({
      id: operationId,
      kind: "cli",
      relatedEntityId: plan.agentId,
      message: "cli.operation.execute",
      terminalStatus: outcome.status,
      error: outcome.error,
      result: mutationResult(operationId, {
        agentId: plan.agentId,
        action: plan.action,
        sourceId: plan.sourceId,
        targetVersion: plan.targetVersion,
        channel: plan.channel,
      }, outcome),
    });
  },

  async prepareCliBulkUpgrade(agentIds: string[]): Promise<OperationTask> {
    const snapshots = webCliEnvironmentSnapshots();
    // A tool is eligible when the backend offered it an upgrade. Everything else is reported as a
    // skip with a reason; a silently shorter list would read as "the rest is up to date".
    const eligible = snapshots.filter(
      (item) => agentIds.includes(item.agentId) && item.allowedActions.some((action) => action.action === "upgrade"),
    );
    const skipped = snapshots
      .filter((item) => agentIds.includes(item.agentId) && !eligible.includes(item))
      .map((item) => ({
        agentId: item.agentId,
        reason: item.conflicts.length > 0 ? "installation-conflict" : "already-current",
      }));
    return createWebMockOperation({
      id: `web-cli-bulk-plan-${nowIso()}`,
      kind: "cli",
      relatedEntityId: null,
      message: "cli.operation.bulk-plan",
      terminalStatus: "succeeded",
      error: null,
      result: { planId: "web-bulk-plan", revision: 1, items: eligible.length, skipped },
    });
  },

  async getCliBulkActionPlan(planId: string): Promise<CliBulkActionPlan> {
    return {
      id: planId,
      revision: 1,
      items: [
        { agentId: "claude-code", planId: "web-plan-claude", sourceId: "npm", currentVersion: "1.2.0", targetVersion: "1.3.0" },
      ],
      skipped: [
        { agentId: "opencode", reason: "installation-conflict" },
        { agentId: "gemini-cli", reason: "already-current" },
      ],
      createdAt: "2026-01-01T00:00:00+00:00",
      expiresAt: "2026-01-01T00:10:00+00:00",
    };
  },

  async executeCliBulkAction(input: ExecuteCliActionInput): Promise<OperationTask> {
    return createWebMockOperation({
      id: `web-cli-bulk-execute-${input.planId}`,
      kind: "cli",
      relatedEntityId: null,
      message: "cli.operation.bulk-execute",
      terminalStatus: "succeeded",
      error: null,
      // Per item, because a batch that half-succeeded is not a batch that succeeded. Both arms
      // of the discriminated union are exercised so a UI can be built against each.
      result: {
        items: [
          {
            status: "completed",
            agentId: "claude-code",
            planId: "web-plan-claude",
            sourceId: "npm",
            targetVersion: "1.3.0",
            outcome: "verified",
            reason: null,
          },
          {
            status: "completed",
            agentId: "codex-cli",
            planId: "web-plan-codex",
            sourceId: "npm",
            targetVersion: "2.0.0",
            outcome: "no-change-failed",
            reason: null,
          },
          {
            status: "skipped",
            agentId: "opencode",
            planId: null,
            sourceId: null,
            targetVersion: null,
            outcome: null,
            reason: "installation-conflict",
          },
        ],
      },
    });
  },

  async runCliDoctor(agentId: string): Promise<OperationTask> {
    return createWebMockOperation({
      id: `web-cli-doctor-${agentId}-${nowIso()}`,
      kind: "cli",
      relatedEntityId: agentId,
      message: "cli.operation.doctor",
      terminalStatus: "succeeded",
      error: null,
      // `unknown` is the honest answer for a runtime that cannot run the tool's own diagnostics.
      result: { agentId, doctor: "unknown" },
    });
  },
};
