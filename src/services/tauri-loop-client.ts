import { invoke } from "@tauri-apps/api/core";
import type { KnownProject } from "../types/agent";
import type {
  LoopAcceptanceResult,
  LoopAdmission,
  LoopAuditAcknowledgement,
  LoopBranchChoice,
  LoopControlEnvelope,
  LoopDefinition,
  LoopProjectChoice,
  LoopReadinessReport,
  LoopRun,
  StartLoopResult,
} from "../types/loop";
import type { LoopWorkbenchService } from "./loop-service";
import { subscribeLoopRunPolling } from "./loop-run-polling";

function envelopeArgument(envelope?: LoopControlEnvelope) {
  return envelope
    ? {
        expectedRevision: envelope.expectedRevision ?? null,
        idempotencyKey: envelope.idempotencyKey ?? null,
        auditAcknowledgementId: envelope.auditAcknowledgementId ?? null,
      }
    : null;
}

export const tauriLoopClient: LoopWorkbenchService = {
  async listLoopProjectChoices() {
    const projects = await invoke<KnownProject[]>("list_known_projects");
    return projects.filter((project) => project.isGit).map<LoopProjectChoice>((project) => ({
      path: project.path,
      displayName: project.displayName,
      available: true,
      simulated: false,
    }));
  },
  listLoopBranches(projectPath) {
    return invoke<LoopBranchChoice[]>("list_loop_branches", { projectPath });
  },
  checkLoopReadiness(definitionId) {
    return invoke<LoopReadinessReport>("check_loop_readiness", { definitionId });
  },
  prepareLoopAdmission(input) {
    return invoke<LoopAdmission>("prepare_loop_admission", {
      input: {
        action: input.action,
        definitionId: input.definitionId ?? null,
        runId: input.runId ?? null,
        expectedRevision: input.expectedRevision,
        clientContext: input.clientContext ?? "desktop",
      },
    });
  },
  acknowledgeLoopAudit(challengeId) {
    return invoke<LoopAuditAcknowledgement>("acknowledge_loop_audit", { challengeId });
  },
  listLoopDefinitions() {
    return invoke<LoopDefinition[]>("list_loop_definitions");
  },
  createLoopDefinition(input) {
    return invoke<LoopDefinition>("create_loop_definition", { input });
  },
  updateLoopDefinition(definitionId, input) {
    return invoke<LoopDefinition>("update_loop_definition", { definitionId, input });
  },
  async deleteLoopDefinition(definitionId) {
    await invoke<void>("delete_loop_definition", { definitionId });
  },
  listLoopRuns(definitionId) {
    return invoke<LoopRun[]>("list_loop_runs", { definitionId: definitionId ?? null });
  },
  getLoopRun(runId) {
    return invoke<LoopRun>("get_loop_run", { runId });
  },
  startLoop(definitionId, envelope) {
    return invoke<StartLoopResult>("start_loop", { definitionId, envelope: envelopeArgument(envelope) });
  },
  pauseLoop(runId) {
    return invoke<LoopRun>("pause_loop", { runId });
  },
  resumeLoop(runId, envelope) {
    return invoke<LoopRun>("resume_loop", { runId, envelope: envelopeArgument(envelope) });
  },
  cancelLoop(runId) {
    return invoke<LoopRun>("cancel_loop", { runId });
  },
  acceptLoop(runId) {
    return invoke<LoopRun>("accept_loop", { runId });
  },
  requestLoopAcceptance(input) {
    return invoke<LoopAcceptanceResult>("request_loop_acceptance", {
      input: {
        runId: input.runId,
        expectedRevision: input.expectedRevision ?? null,
        expectedScopeDigest: input.expectedScopeDigest ?? null,
        expectedEvidenceId: input.expectedEvidenceId ?? null,
        idempotencyKey: input.idempotencyKey ?? null,
      },
    });
  },
  continueLoop(input) {
    return invoke<LoopRun>("continue_loop", {
      input: { runId: input.runId, feedback: input.feedback, envelope: envelopeArgument(input.envelope) },
    });
  },
  rejectLoop(runId) {
    return invoke<LoopRun>("reject_loop", { runId });
  },
  async subscribeLoopEvents(runId, handler) {
    return subscribeLoopRunPolling(() => invoke<LoopRun>("get_loop_run", { runId }), handler);
  },
};
