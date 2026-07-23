import { invoke } from "@tauri-apps/api/core";
import type {
  ExecutionObservationCapability,
  ExecutionRunPage,
  ExecutionRunSummary,
  ExecutionTimeline,
  ObservabilitySettings,
} from "../types/execution-observability";
import type { ExecutionObservabilityService } from "./execution-observability-service";

export const tauriExecutionObservabilityClient: ExecutionObservabilityService = {
  getSettings() {
    return invoke<ObservabilitySettings>("get_observability_settings");
  },

  updateSettings(settings) {
    return invoke<ObservabilitySettings>("update_observability_settings", { settings });
  },

  listRuns(query) {
    return invoke<ExecutionRunPage>("list_execution_runs", {
      request: { limit: query.limit, pageToken: query.pageToken ?? null },
      sessionId: query.sessionId ?? null,
    });
  },

  getRun(runId) {
    return invoke<ExecutionRunSummary>("get_execution_run", { runId });
  },

  getTimeline(runId) {
    return invoke<ExecutionTimeline>("get_execution_timeline", { runId });
  },

  getObservationCapabilities() {
    return invoke<ExecutionObservationCapability[]>("get_execution_observation_capabilities");
  },
};
