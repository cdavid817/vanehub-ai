export type GenerationArtifactKind =
  | "overlay_learn_block"
  | "overlay_exact_patch"
  | "new_skill";

export type GenerationJobStatus =
  | "requested"
  | "blocked_consent"
  | "queued"
  | "running"
  | "cancel_requested"
  | "cancelled"
  | "failed"
  | "completed"
  | "superseded";

export type GenerationStageKind =
  | "freeze_input"
  | "inspect_target"
  | "build_dossier"
  | "plan_mutation"
  | "synthesize_structured_draft"
  | "validate_and_simulate"
  | "package_for_governance";

export interface GenerationPolicy {
  workspaceId: string;
  enabled: boolean;
  disclosureVersion: string;
  providerProfileId?: string;
  modelId?: string;
  allowedArtifactKinds: GenerationArtifactKind[];
  dailyModelCalls: number;
  dailyInputTokens: number;
  dailyOutputTokens: number;
  failedCancelledRetentionDays: number;
  completedPackageRetentionDays: number;
  revision: number;
  policyHash: string;
}

export interface UpdateGenerationPolicyInput {
  workspaceId: string;
  expectedRevision: number;
  enabled: boolean;
  disclosureVersion: string;
  providerProfileId?: string;
  modelId?: string;
  allowedArtifactKinds: GenerationArtifactKind[];
}

export interface GenerationUsage {
  elapsedMs: number;
  modelCalls: number;
  toolCalls: number;
  inputTokens: number;
  outputTokens: number;
  validationRepairs: number;
}

export interface GenerationStageAttempt {
  attemptId: string;
  stage: GenerationStageKind;
  attempt: number;
  status: "pending" | "running" | "succeeded" | "failed" | "cancelled" | "superseded";
  inputHash: string;
  outputHash?: string;
  usage: GenerationUsage;
  safeFailureCode?: string;
  startedAt: string;
  completedAt?: string;
}

export interface GenerationJobSummary {
  jobId: string;
  requestId: string;
  workspaceId?: string;
  seedId: string;
  assessmentAttemptId: string;
  status: GenerationJobStatus;
  artifactKind?: GenerationArtifactKind;
  currentStage?: GenerationStageKind;
  usage: GenerationUsage;
  safeFailureCode?: string;
  handoffStatus?: string;
  inputWitnessHash?: string;
  supersedesJobId?: string;
  createdAt: string;
  updatedAt: string;
}

export interface GenerationDraftProjection {
  draftId: string;
  generationAttempt: number;
  artifactKind: GenerationArtifactKind;
  mediaType: string;
  renderedContent: string;
  sizeBytes: number;
  contentHash: string;
  permanentlyManual: true;
  citations: Array<{ claimId: string; dossierSection: string; sourceId: string }>;
}

export interface GenerationValidationProjection {
  validationId: string;
  status: string;
  checks: unknown[];
  previewWitnessHash?: string;
  reportHash: string;
  repairAttempt: number;
}

export interface GenerationJobDetail extends GenerationJobSummary {
  stages: GenerationStageAttempt[];
  dossierId?: string;
  dossierRevision?: number;
  dossierHash?: string;
  draftId?: string;
  artifactHash?: string;
  validationId?: string;
  previewWitnessHash?: string;
  draft?: GenerationDraftProjection;
  validation?: GenerationValidationProjection;
  permanentlyManual: true;
}

export interface GenerationQuery {
  workspaceId?: string;
  skillId?: string;
  status?: GenerationJobStatus;
  limit?: number;
  cursor?: string;
}

export interface GenerationPage<T> {
  items: T[];
  nextCursor?: string;
}

export interface DossierSectionPage {
  dossierId: string;
  dossierRevision: number;
  ordinal: number;
  kind: string;
  status: string;
  records: unknown[];
  sourceWitnesses: unknown[];
  sourceLinks: Array<{ linkKind: string; linkedId: string; linkedRevision: string; witnessHash: string }>;
  unavailableReasonCode?: string;
  truncation: { complete: boolean; omittedRecords: number; omittedBytes: number };
  sectionHash: string;
  nextCursor?: string;
  pageComplete: boolean;
}

export interface GenerationProvenance {
  jobId: string;
  modelCalls: Array<Record<string, unknown>>;
  toolReceipts: Array<Record<string, unknown>>;
  validations: Array<Record<string, unknown>>;
}

export interface GenerationQuarantineSummary {
  proposalId: string;
  jobId: string;
  status: string;
  candidateId: string;
  scope: "user" | "project";
  workspaceId?: string;
  artifactHash: string;
  catalogWitnessHash: string;
  revision: number;
}

export interface GenerationExportInput {
  dossierId: string;
  format: "json" | "markdown";
}

export interface GenerationExportResult {
  exportId: string;
  status: "exported" | "cancelled";
  contentHash?: string;
  sizeBytes?: number;
  exportedFileRemainsUserManaged: boolean;
}

export interface RegenerateGenerationInput {
  jobId: string;
  expectedInputWitnessHash: string;
  requestId: string;
}

// Failure attention is not a generation-channel kind: failed jobs reach the user through the
// orchestration run_attention notification and the durable system-activity notification path,
// so this union carries only the events the generation channel actually emits.
export type GenerationNotificationKind = "review_ready" | "cancelled" | "superseded";

export interface GenerationNotificationEvent {
  schemaVersion: 1;
  eventId: string;
  eventKind: GenerationNotificationKind;
  jobId: string;
  workspaceId: string;
  seedId: string;
  safeFailureCode?: string;
}

export interface SkillGenerationService {
  getGenerationPolicy(workspaceId: string): Promise<GenerationPolicy>;
  updateGenerationPolicy(input: UpdateGenerationPolicyInput): Promise<GenerationPolicy>;
  listGenerationJobs(input: GenerationQuery): Promise<GenerationPage<GenerationJobSummary>>;
  getGenerationJob(jobId: string): Promise<GenerationJobDetail | null>;
  cancelGenerationJob(jobId: string): Promise<GenerationJobDetail>;
  regenerateGenerationJob(input: RegenerateGenerationInput): Promise<GenerationJobDetail>;
  getGenerationDossierSection(
    dossierId: string,
    ordinal: number,
    cursor?: string,
    limit?: number,
  ): Promise<DossierSectionPage>;
  getGenerationProvenance(jobId: string): Promise<GenerationProvenance>;
  listGenerationQuarantine(input: GenerationQuery): Promise<GenerationPage<GenerationQuarantineSummary>>;
  exportGenerationDossier(input: GenerationExportInput): Promise<GenerationExportResult>;
  handoffGenerationPackage(jobId: string): Promise<GenerationJobDetail>;
  subscribeGenerationNotifications(handler: (event: GenerationNotificationEvent) => void): Promise<() => void>;
}
