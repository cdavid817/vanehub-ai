import type {
  ApplyDelegationChangesInput,
  ArtifactChunk,
  ArtifactDetail,
  ArtifactPage,
  BrowserHandoffState,
  BuiltinToolOperation,
  BuiltinToolOperationEvent,
  BuiltinToolReadiness,
  ChangeSetReview,
  DelegationAttemptSummary,
  DelegationRecoveryResult,
  DelegationReport,
  DownloadArtifactInput,
  DownloadArtifactResult,
  ListArtifactsInput,
  ListBuiltinToolOperationsInput,
  PublishArtifactInput,
  ReadArtifactInput,
  StartDelegationInput,
} from "../types/builtin-tools";

export interface BuiltinToolService {
  getBuiltinToolReadiness(agentId: string): Promise<BuiltinToolReadiness>;
  getBuiltinToolOperation(operationId: string): Promise<BuiltinToolOperation>;
  listBuiltinToolOperations(input: ListBuiltinToolOperationsInput): Promise<BuiltinToolOperation[]>;
  cancelBuiltinToolOperation(operationId: string): Promise<BuiltinToolOperation>;
  subscribeBuiltinToolOperations(
    sessionId: string,
    listener: (event: BuiltinToolOperationEvent) => void,
  ): Promise<() => void>;
  listArtifacts(input: ListArtifactsInput): Promise<ArtifactPage>;
  getArtifact(artifactId: string): Promise<ArtifactDetail>;
  readArtifact(input: ReadArtifactInput): Promise<ArtifactChunk>;
  publishArtifact(input: PublishArtifactInput): Promise<ArtifactDetail>;
  downloadArtifact(input: DownloadArtifactInput): Promise<DownloadArtifactResult>;
  startDelegation(input: StartDelegationInput): Promise<DelegationAttemptSummary>;
  listDelegationAttempts(sessionId: string): Promise<DelegationAttemptSummary[]>;
  getDelegationReport(attemptId: string): Promise<DelegationReport>;
  getChangeSetReview(artifactId: string): Promise<ChangeSetReview>;
  applyDelegationChanges(input: ApplyDelegationChangesInput): Promise<BuiltinToolOperation>;
  getDelegationRecovery(operationId: string): Promise<DelegationRecoveryResult>;
  getBrowserHandoff(operationId: string): Promise<BrowserHandoffState>;
  beginBrowserHandoff(operationId: string): Promise<BrowserHandoffState>;
  resumeBrowserAutomation(operationId: string, ownershipToken: string): Promise<BrowserHandoffState>;
}
