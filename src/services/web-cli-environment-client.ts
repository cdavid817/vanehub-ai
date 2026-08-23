import type { CliToolService } from "./cli-service";
import { createWebMockOperation } from "./web-operation-client";
import { nowIso } from "./web-mock-clock";
import {
  WEB_CLI_FIXED_PLAN_IDS,
  WEB_CLI_REFUSAL_TARGETS,
  webCliActionPlan,
  webCliBulkActionPlan,
  webCliBulkItemResults,
  webCliEnvironmentSnapshots,
  webCliOutcomeFor,
} from "./web-cli-environment-fixtures";
import { CliEnvironmentRejection } from "../types/cli-environment";
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

function mutationResult(
  operationId: string,
  input: PrepareCliActionInput,
  outcome: ReturnType<typeof webCliOutcomeFor>,
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

/** Which already-unusable plan a sentinel target asks for, if any. */
function refusalPlanId(targetVersion: string | null): string | null {
  if (targetVersion === WEB_CLI_REFUSAL_TARGETS.expired) return WEB_CLI_FIXED_PLAN_IDS.expired;
  if (targetVersion === WEB_CLI_REFUSAL_TARGETS.stale) return WEB_CLI_FIXED_PLAN_IDS.stale;
  return null;
}

/** Plans the mock always answers for, one per refusal the desktop runtime can produce. */
function fixedPlan(planId: string): CliActionPlan | null {
  switch (planId) {
    case WEB_CLI_FIXED_PLAN_IDS.expired:
      return webCliActionPlan({ id: planId, state: "expired" });
    case WEB_CLI_FIXED_PLAN_IDS.consumed:
      return webCliActionPlan({ id: planId, state: "completed" });
    // Both read as ordinary drafts. What makes the stale one stale happens at execution, not here.
    case WEB_CLI_FIXED_PLAN_IDS.stale:
    case WEB_CLI_FIXED_PLAN_IDS.draft:
      return webCliActionPlan({ id: planId });
    default:
      return null;
  }
}

let preparedPlan: CliActionPlan | null = null;

/**
 * The revision this plan carries at the moment execution admits it.
 *
 * One plan id is always revised between review and execution, whatever revision the review showed.
 * Modelling that by counting reads made the outcome depend on how many times the caller happened
 * to refetch, which is not a property of the environment being modelled.
 */
function revisionAtExecution(plan: CliActionPlan, reviewed: number): number {
  return plan.id === WEB_CLI_FIXED_PLAN_IDS.stale ? reviewed + 1 : plan.revision;
}

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
      cancellable: true,
      // A real refresh probes several CLIs with bounded timeouts and takes seconds. Settling in
      // under a second would make the refreshing state, and the chance to cancel it, invisible.
      settleAfterMs: 2600,
    });
  },

  async prepareCliAction(input: PrepareCliActionInput): Promise<OperationTask> {
    // The chosen source, channel, and target land on the plan unchanged. Substituting any of them
    // here would reproduce, in the mock, the defect the action plan exists to prevent.
    preparedPlan = webCliActionPlan({
      id: refusalPlanId(input.targetVersion) ?? `web-plan-${input.agentId}-${nowIso()}`,
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
    // Refused before anything runs, in the same shape and with the same categories the desktop
    // command layer rejects with, so a caller cannot handle one runtime and drop the other.
    if (revisionAtExecution(plan, input.expectedRevision) !== input.expectedRevision) {
      throw new CliEnvironmentRejection("plan-revision-mismatch", true);
    }
    if (plan.state !== "draft") {
      throw new CliEnvironmentRejection(plan.state === "expired" ? "plan-expired" : "plan-consumed", true);
    }
    const outcome = webCliOutcomeFor(plan.targetVersion);
    const operationId = `web-cli-execute-${plan.id}`;
    return createWebMockOperation({
      id: operationId,
      kind: "cli",
      relatedEntityId: plan.agentId,
      message: "cli.operation.execute",
      terminalStatus: outcome.status,
      error: outcome.error,
      cancellable: true,
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
    return webCliBulkActionPlan(planId);
  },

  async executeCliBulkAction(input: ExecuteCliActionInput): Promise<OperationTask> {
    return createWebMockOperation({
      id: `web-cli-bulk-execute-${input.planId}`,
      kind: "cli",
      relatedEntityId: null,
      message: "cli.operation.bulk-execute",
      terminalStatus: "succeeded",
      error: null,
      cancellable: true,
      // Per item, because a batch that half-succeeded is not a batch that succeeded. Both arms of
      // the discriminated union are exercised so a UI can be built against each.
      result: { items: webCliBulkItemResults() },
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
