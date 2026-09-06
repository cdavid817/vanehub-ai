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
  /**
   * One run's timeline. `eventPageToken` resumes the event list after a cursor a previous
   * response reported; the run and its spans are returned identically either way.
   */
  getTimeline(runId: string, eventPageToken?: string | null): Promise<ExecutionTimeline>;
  getObservationCapabilities(): Promise<ExecutionObservationCapability[]>;
}
