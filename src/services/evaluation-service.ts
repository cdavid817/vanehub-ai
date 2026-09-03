import type { EvaluationArena, EvaluationArenaPage, EvaluationArenaQuery, EvaluationAttempt, EvaluationExport, EvaluationTask, StartEvaluationInput } from "../types/evaluation";

export interface EvaluationService {
  listEvaluationTasks(): Promise<EvaluationTask[]>;
  startEvaluation(input: StartEvaluationInput): Promise<EvaluationArena>;
  /** 18.6: real service-side pagination -- an omitted `query` fetches the first page at the
   *  service's own default page size (see `tauri-agent-client.ts`/`web-evaluation-client.ts`). */
  listEvaluationArenas(query?: EvaluationArenaQuery): Promise<EvaluationArenaPage>;
  getEvaluationArena(arenaId: string): Promise<EvaluationArena>;
  cancelEvaluation(arenaId: string): Promise<EvaluationArena>;
  getEvaluationAttempt(attemptId: string): Promise<EvaluationAttempt>;
  exportEvaluation(arenaId: string): Promise<EvaluationExport>;
}
