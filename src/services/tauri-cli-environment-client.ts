import { invoke } from "@tauri-apps/api/core";
import type { CliToolService } from "./cli-service";
import type {
  CliActionPlan,
  CliBulkActionPlan,
  CliEnvironmentSnapshot,
  ExecuteCliActionInput,
  PrepareCliActionInput,
} from "../types/cli-environment-snapshot";
import type { OperationTask } from "../types/operation";

/**
 * The desktop half of the source-aware CLI surface.
 *
 * Mirrors `web-cli-environment-client.ts` so the two runtimes stay side by side and a method
 * missing from one is obvious. Split out of `tauri-agent-client.ts` rather than added to it: that
 * file is already at its line budget, and a budget is raised for a reason, not for an import.
 *
 * Every method is a one-to-one relay. Nothing here builds a command, compares versions, or infers
 * what a source can do -- the backend decided all three, and deciding them again on this side is
 * the divergence this change exists to remove.
 */

/** What a `prepare_*` or `execute_*` command returns before its work completes. */
interface CliOperationHandle {
  operationId: string;
}

/**
 * Turns a returned operation id into the task shape callers poll.
 *
 * Deliberately `queued`: the backend has accepted the work and started nothing observable yet.
 * Reporting `running` here would claim a phase the operation has not entered.
 */
function startedCliOperation(
  handle: CliOperationHandle,
  relatedEntityId: string | null,
): OperationTask {
  return {
    id: handle.operationId,
    kind: "cli",
    status: "queued",
    relatedEntityId,
    message: null,
    logs: [],
    result: null,
    error: null,
    createdAt: "",
    updatedAt: "",
  };
}

export const tauriCliEnvironmentClient: CliToolService = {
  listCliEnvironments() {
    return invoke<CliEnvironmentSnapshot[]>("list_cli_environments");
  },

  async refreshCliEnvironments(agentIds: string[], forceCatalog: boolean) {
    const handle = await invoke<CliOperationHandle>("refresh_cli_environment", {
      agentIds,
      forceCatalog,
    });
    return startedCliOperation(handle, agentIds.length === 1 ? agentIds[0] : null);
  },

  async prepareCliAction(input: PrepareCliActionInput) {
    // Passed through verbatim. A substitution here -- latest for a chosen version, a fallback
    // source for an unavailable one -- is exactly what the action plan exists to prevent.
    const handle = await invoke<CliOperationHandle>("prepare_cli_action", {
      agentId: input.agentId,
      action: input.action,
      sourceId: input.sourceId,
      targetVersion: input.targetVersion,
      channel: input.channel,
    });
    return startedCliOperation(handle, input.agentId);
  },

  getCliActionPlan(planId: string) {
    return invoke<CliActionPlan>("get_cli_action_plan", { planId });
  },

  async executeCliAction(input: ExecuteCliActionInput) {
    // A plan id and the revision the user saw. Nothing else crosses, so nothing else can be
    // substituted between review and execution.
    const handle = await invoke<CliOperationHandle>("execute_cli_action", {
      planId: input.planId,
      expectedRevision: input.expectedRevision,
    });
    return startedCliOperation(handle, null);
  },

  async prepareCliBulkUpgrade(agentIds: string[]) {
    const handle = await invoke<CliOperationHandle>("prepare_cli_bulk_action", { agentIds });
    return startedCliOperation(handle, null);
  },

  getCliBulkActionPlan(planId: string) {
    return invoke<CliBulkActionPlan>("get_cli_bulk_action_plan", { planId });
  },

  async executeCliBulkAction(input: ExecuteCliActionInput) {
    const handle = await invoke<CliOperationHandle>("execute_cli_bulk_action", {
      planId: input.planId,
      expectedRevision: input.expectedRevision,
    });
    return startedCliOperation(handle, null);
  },

  async runCliDoctor(agentId: string) {
    const handle = await invoke<CliOperationHandle>("run_cli_doctor", { agentId });
    return startedCliOperation(handle, agentId);
  },
};
