import type {
  ContinueLoopInput,
  LoopDefinition,
  LoopEvent,
  LoopRun,
  SaveLoopDefinitionInput,
  StartLoopResult,
} from "../types/loop";

export interface LoopService {
  listLoopDefinitions(): Promise<LoopDefinition[]>;
  createLoopDefinition(input: SaveLoopDefinitionInput): Promise<LoopDefinition>;
  updateLoopDefinition(definitionId: string, input: SaveLoopDefinitionInput): Promise<LoopDefinition>;
  deleteLoopDefinition(definitionId: string): Promise<void>;
  listLoopRuns(definitionId?: string): Promise<LoopRun[]>;
  getLoopRun(runId: string): Promise<LoopRun>;
  startLoop(definitionId: string): Promise<StartLoopResult>;
  pauseLoop(runId: string): Promise<LoopRun>;
  resumeLoop(runId: string): Promise<LoopRun>;
  cancelLoop(runId: string): Promise<LoopRun>;
  acceptLoop(runId: string): Promise<LoopRun>;
  continueLoop(input: ContinueLoopInput): Promise<LoopRun>;
  rejectLoop(runId: string): Promise<LoopRun>;
  subscribeLoopEvents(runId: string, handler: (event: LoopEvent) => void): Promise<() => void>;
}
