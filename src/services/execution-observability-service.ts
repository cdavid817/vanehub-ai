import type {
  ExecutionObservationCapability,
  ExecutionRunPage,
  ExecutionRunSummary,
  ExecutionTimeline,
  ObservabilitySettings,
  PageRequest,
} from "../types/execution-observability";

export interface ExecutionRunQuery extends PageRequest {
  sessionId?: string | null;
}

export interface ExecutionObservabilityService {
  getSettings(): Promise<ObservabilitySettings>;
  updateSettings(settings: ObservabilitySettings): Promise<ObservabilitySettings>;
  listRuns(query: ExecutionRunQuery): Promise<ExecutionRunPage>;
  getRun(runId: string): Promise<ExecutionRunSummary>;
  getTimeline(runId: string): Promise<ExecutionTimeline>;
  getObservationCapabilities(): Promise<ExecutionObservationCapability[]>;
}
