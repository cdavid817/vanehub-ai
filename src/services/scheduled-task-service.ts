import type {
  AutomaticArchivalSettings,
  CreateScheduledTaskInput,
  RunScheduledTaskNowResult,
  ScheduledTask,
  ScheduledTaskRunPage,
  ScheduledTaskRunQuery,
  SetScheduledTaskEnabledInput,
  UpdateScheduledTaskInput,
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
  /** 19.11: real service-side pagination -- an omitted `query` fetches the first page at the
   *  service's own default page size (see `tauri-agent-client.ts`/`web-scheduled-task-client.ts`),
   *  mirroring `EvaluationService.listEvaluationArenas`'s own identical contract (18.6). */
  listScheduledTaskRuns(taskId: string, query?: ScheduledTaskRunQuery): Promise<ScheduledTaskRunPage>;
  createScheduledTask(input: CreateScheduledTaskInput): Promise<ScheduledTask>;
  setScheduledTaskEnabled(input: SetScheduledTaskEnabledInput): Promise<ScheduledTask>;
  updateScheduledTask(input: UpdateScheduledTaskInput): Promise<ScheduledTask>;
  deleteScheduledTask(taskId: string): Promise<void>;
  runScheduledTaskNow(taskId: string): Promise<RunScheduledTaskNowResult>;
}

export interface ContextQualityService {
  listContextQualityHistory(input: ContextQualityHistoryQuery): Promise<ContextQualityHistoryPage>;
  getContextQualitySummary(input: ContextQualitySummaryQuery): Promise<ContextQualitySummary>;
  listContextEvidenceManifests(input: ContextEvidenceManifestQuery): Promise<ContextEvidenceManifestPage>;
  getContextEvidenceManifest(generationId: string): Promise<ContextEvidenceManifest | null>;
}
