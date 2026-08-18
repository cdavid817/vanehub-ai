import type {
  AutomaticArchivalSettings,
  CreateScheduledTaskInput,
  ScheduledTask,
  ScheduledTaskRun,
  SetScheduledTaskEnabledInput,
} from "../types/agent";
import type {
  ContextQualityHistoryPage,
  ContextQualityHistoryQuery,
  ContextQualitySummary,
  ContextQualitySummaryQuery,
} from "../types/context-quality";
import type {
  ContextEvidenceManifest,
  ContextEvidenceManifestPage,
  ContextEvidenceManifestQuery,
} from "../types/context-engine";

export interface ScheduledTaskService {
  getAutomaticArchivalSettings(): Promise<AutomaticArchivalSettings>;
  saveAutomaticArchivalSettings(input: AutomaticArchivalSettings): Promise<AutomaticArchivalSettings>;
  listScheduledTasks(): Promise<ScheduledTask[]>;
  listScheduledTaskRuns(taskId: string): Promise<ScheduledTaskRun[]>;
  createScheduledTask(input: CreateScheduledTaskInput): Promise<ScheduledTask>;
  setScheduledTaskEnabled(input: SetScheduledTaskEnabledInput): Promise<ScheduledTask>;
  deleteScheduledTask(taskId: string): Promise<void>;
}

export interface ContextQualityService {
  listContextQualityHistory(input: ContextQualityHistoryQuery): Promise<ContextQualityHistoryPage>;
  getContextQualitySummary(input: ContextQualitySummaryQuery): Promise<ContextQualitySummary>;
  listContextEvidenceManifests(input: ContextEvidenceManifestQuery): Promise<ContextEvidenceManifestPage>;
  getContextEvidenceManifest(generationId: string): Promise<ContextEvidenceManifest | null>;
}
