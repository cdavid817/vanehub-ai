import { invoke } from "@tauri-apps/api/core";
import type { KnownProject } from "../types/agent";
import type {
  LoopBranchChoice,
  LoopDefinition,
  LoopProjectChoice,
  LoopReadinessReport,
  LoopRun,
  StartLoopResult,
} from "../types/loop";
import type { LoopWorkbenchService } from "./loop-service";
import { subscribeLoopRunPolling } from "./loop-run-polling";

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
  startLoop(definitionId) {
    return invoke<StartLoopResult>("start_loop", { definitionId });
  },
  pauseLoop(runId) {
    return invoke<LoopRun>("pause_loop", { runId });
  },
  resumeLoop(runId) {
    return invoke<LoopRun>("resume_loop", { runId });
  },
  cancelLoop(runId) {
    return invoke<LoopRun>("cancel_loop", { runId });
  },
  acceptLoop(runId) {
    return invoke<LoopRun>("accept_loop", { runId });
  },
  continueLoop(input) {
    return invoke<LoopRun>("continue_loop", { input });
  },
  rejectLoop(runId) {
    return invoke<LoopRun>("reject_loop", { runId });
  },
  async subscribeLoopEvents(runId, handler) {
    return subscribeLoopRunPolling(() => invoke<LoopRun>("get_loop_run", { runId }), handler);
  },
};
