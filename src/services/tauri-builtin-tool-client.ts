import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BuiltinToolService } from "./builtin-tool-service";
import type { BuiltinToolOperationEvent } from "../types/builtin-tools";

export const tauriBuiltinToolClient: BuiltinToolService = {
  getBuiltinToolReadiness: (agentId) =>
    invoke("get_builtin_tool_readiness", { agentId }),
  getBuiltinToolOperation: (operationId) =>
    invoke("get_builtin_tool_operation", { operationId }),
  listBuiltinToolOperations: (input) =>
    invoke("list_builtin_tool_operations", { input }),
  cancelBuiltinToolOperation: (operationId) =>
    invoke("cancel_builtin_tool_operation", { operationId }),
  async subscribeBuiltinToolOperations(sessionId, listener) {
    return listen<BuiltinToolOperationEvent>("builtin-tool-operation", (event) => {
      const payload = event.payload;
      if (payload.kind === "removed" || payload.operation.sessionId === sessionId) listener(payload);
    });
  },
  listArtifacts: (input) => invoke("list_artifacts", { input }),
  getArtifact: (artifactId) => invoke("get_artifact", { artifactId }),
  readArtifact: (input) => invoke("read_artifact", { input }),
  publishArtifact: (input) => invoke("publish_artifact", { input }),
  downloadArtifact: (input) => invoke("download_artifact", { input }),
  startDelegation: (input) => invoke("start_delegation", { input }),
  listDelegationAttempts: (sessionId) =>
    invoke("list_delegation_attempts", { sessionId }),
  getDelegationReport: (attemptId) => invoke("get_delegation_report", { attemptId }),
  getChangeSetReview: (artifactId) => invoke("get_change_set_review", { artifactId }),
  applyDelegationChanges: (input) => invoke("apply_delegation_changes", { input }),
  getDelegationRecovery: (operationId) =>
    invoke("get_delegation_recovery", { operationId }),
  getBrowserHandoff: (operationId) => invoke("get_browser_handoff", { operationId }),
  beginBrowserHandoff: (operationId) => invoke("begin_browser_handoff", { operationId }),
  resumeBrowserAutomation: (operationId, ownershipToken) =>
    invoke("resume_browser_automation", { operationId, ownershipToken }),
};
