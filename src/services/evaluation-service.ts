import type { EvaluationArena, EvaluationAttempt, EvaluationExport, EvaluationTask, StartEvaluationInput } from "../types/evaluation";

export interface EvaluationService {
  listEvaluationTasks(): Promise<EvaluationTask[]>;
  startEvaluation(input: StartEvaluationInput): Promise<EvaluationArena>;
  listEvaluationArenas(): Promise<EvaluationArena[]>;
  getEvaluationArena(arenaId: string): Promise<EvaluationArena>;
  cancelEvaluation(arenaId: string): Promise<EvaluationArena>;
  getEvaluationAttempt(attemptId: string): Promise<EvaluationAttempt>;
  exportEvaluation(arenaId: string): Promise<EvaluationExport>;
}
