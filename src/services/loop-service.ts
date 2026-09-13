import type {
  ContinueLoopInput,
  LoopAcceptanceResult,
  LoopAdmission,
  LoopAuditAcknowledgement,
  LoopBranchChoice,
  LoopControlEnvelope,
  LoopDefinition,
  LoopEvent,
  LoopProjectChoice,
  LoopReadinessReport,
  LoopRun,
  PrepareLoopAdmissionInput,
  RequestLoopAcceptanceInput,
  SaveLoopDefinitionInput,
  StartLoopResult,
} from "../types/loop";

export interface LoopReadinessService {
  listLoopProjectChoices(): Promise<LoopProjectChoice[]>;
  listLoopBranches(projectPath: string): Promise<LoopBranchChoice[]>;
  checkLoopReadiness(definitionId: string): Promise<LoopReadinessReport>;
  /** Facts plus a short-lived challenge for an audit-mode transition. Creates no run or session. */
  prepareLoopAdmission(input: PrepareLoopAdmissionInput): Promise<LoopAdmission>;
  /** Turns a challenge into a one-use receipt that only the intended transition can consume. */
  acknowledgeLoopAudit(challengeId: string): Promise<LoopAuditAcknowledgement>;
}

export interface LoopService {
  listLoopDefinitions(): Promise<LoopDefinition[]>;
  createLoopDefinition(input: SaveLoopDefinitionInput): Promise<LoopDefinition>;
  updateLoopDefinition(definitionId: string, input: SaveLoopDefinitionInput): Promise<LoopDefinition>;
  deleteLoopDefinition(definitionId: string): Promise<void>;
  listLoopRuns(definitionId?: string): Promise<LoopRun[]>;
  getLoopRun(runId: string): Promise<LoopRun>;
  startLoop(definitionId: string, envelope?: LoopControlEnvelope): Promise<StartLoopResult>;
  pauseLoop(runId: string): Promise<LoopRun>;
  resumeLoop(runId: string, envelope?: LoopControlEnvelope): Promise<LoopRun>;
  cancelLoop(runId: string): Promise<LoopRun>;
  /** Routes through the native sealed-evidence gate; the run stays awaiting until it commits. */
  acceptLoop(runId: string): Promise<LoopRun>;
  requestLoopAcceptance(input: RequestLoopAcceptanceInput): Promise<LoopAcceptanceResult>;
  continueLoop(input: ContinueLoopInput): Promise<LoopRun>;
  rejectLoop(runId: string): Promise<LoopRun>;
  subscribeLoopEvents(runId: string, handler: (event: LoopEvent) => void): Promise<() => void>;
}

export interface LoopWorkbenchService extends LoopService, LoopReadinessService {}
