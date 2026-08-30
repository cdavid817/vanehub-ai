export type EvolutionPolicyMode = "off" | "observe" | "enabled";
export type EvolutionRunStatus =
  | "requested" | "waiting_idle" | "running" | "partial" | "completed"
  | "failed" | "cancel_requested" | "cancelled" | "recovered";

export interface EvolutionConsent {
  disclosureVersion: string;
  disclosureHash: string;
  acceptedAtMs: number;
}

export interface EvolutionPolicy {
  workspaceId: string;
  mode: EvolutionPolicyMode;
  allowedSkillIds: string[];
  consent: EvolutionConsent | null;
  revision: number;
  updatedAtMs: number;
  mockProvenance?: "web_simulation";
}

export interface EvolutionPolicyUpdate {
  workspaceId: string;
  expectedRevision: number;
  mode: EvolutionPolicyMode;
  allowedSkillIds: string[];
  acknowledgeCurrentDisclosure: boolean;
}

export interface EvolutionSchedulerOverview {
  workspaceId: string;
  mode: EvolutionPolicyMode;
  pendingTriggers: number;
  activeRunId: string | null;
  idleGate: "ready" | "waiting" | "unavailable";
  automaticMutationAvailable: boolean;
  triggerCounters: EvolutionTriggerCounters;
  idle: EvolutionIdleProjection;
  mockProvenance?: "web_simulation";
}

export interface EvolutionTriggerCounters {
  startupRecovery: number;
  periodicMaintenance: number;
  applicationIdleTransition: number;
  agentRunCompletion: number;
  conversationCompletion: number;
  explicitFeedbackCommit: number;
  verificationCompletion: number;
  delegatedUtilityCompletion: number;
  relevantPolicyOrSkillChange: number;
  manualRunRequest: number;
}

export interface EvolutionIdleProjection {
  state: "ready" | "waiting" | "unavailable";
  safeReasons: string[];
}

export interface EvolutionRunBudgetProjection {
  wallTimeMs: number;
  evidenceItems: number;
  seedGroups: number;
  assessments: number;
  modelCalls: number;
  notifications: number;
  automaticMutations: number;
}

export interface EvolutionRunUsageProjection {
  elapsedMs: number;
  evidenceItems: number;
  seedGroups: number;
  assessments: number;
  modelCalls: number;
  notifications: number;
  automaticMutations: number;
}

export interface EvolutionRunSummary {
  runId: string;
  workspaceId: string;
  status: EvolutionRunStatus;
  currentStage: string | null;
  policyWitnessHash: string;
  safeFailureCode: string | null;
  budget: EvolutionRunBudgetProjection;
  usage: EvolutionRunUsageProjection;
  revision: number;
  createdAtMs: number;
  updatedAtMs: number;
  mockProvenance?: "web_simulation";
}

export interface EvolutionStageSummary {
  stageId: string;
  runId: string;
  stage: string;
  attempt: number;
  status: string;
  safeFailureCode: string | null;
  startedAtMs: number | null;
  completedAtMs: number | null;
}

export interface EvolutionCheckpointSummary {
  checkpointId: string;
  runId: string;
  stage: string;
  status: string;
  cursorRecordId: string | null;
  continuationNotBeforeMs: number | null;
  committedAtMs: number;
}

export interface EvolutionRunDetail extends EvolutionRunSummary {
  stages: EvolutionStageSummary[];
  checkpoints: EvolutionCheckpointSummary[];
}

export interface EvolutionEligibilityPredicate {
  condition: string;
  passed: boolean;
  safeReasonCode: string | null;
  witnessHash: string | null;
}

export interface EvolutionEligibilitySummary {
  eligibilityId: string;
  runId: string;
  draftId: string;
  targetSkillId: string;
  draftProvenance: "deterministic_authorized_correction";
  preflightState: "not_issued" | "active" | "consumed" | "expired";
  result: "ineligible" | "waiting" | "routed_to_curator" | "would_apply" | "eligible";
  proofHash: string;
  overlayPreviewHash: string | null;
  evaluatedAtMs: number;
  predicates: EvolutionEligibilityPredicate[];
  mockProvenance?: "web_simulation";
}

export interface EvolutionApplicationSummary {
  applicationId: string;
  runId: string;
  eligibilityId: string;
  targetSkillId: string;
  curatorApplicationId: string;
  overlayApplicationId: string;
  actor: "system_policy";
  committedAtMs: number;
  mockProvenance?: "web_simulation";
}

export interface EvolutionProbationSummary {
  probationId: string;
  applicationId: string;
  workspaceId: string;
  skillId: string;
  status: "active" | "healthy" | "regressed" | "expired" | "suspended";
  startsAtMs: number;
  endsAtMs: number;
  revision: number;
  mockProvenance?: "web_simulation";
}

export interface EvolutionBreakerSummary {
  breakerId: string;
  workspaceId: string;
  skillId: string | null;
  status: "closed" | "open" | "awaiting_health" | "awaiting_acknowledgement";
  safeCauseCode: string | null;
  healthCheckVersion: string;
  healthProbePassed: boolean;
  revision: number;
  updatedAtMs: number;
  mockProvenance?: "web_simulation";
}

export interface EvolutionPage<T> {
  items: T[];
  nextCursor: string | null;
}

export interface EvolutionQuery {
  workspaceId: string;
  cursor?: string;
  limit?: number;
}

export interface EvolutionRunRequestReceipt {
  requestId: string;
  queued: boolean;
  mockProvenance?: "web_simulation";
}

export interface EvolutionRunMutationReceipt {
  runId: string;
  status: EvolutionRunStatus;
  revision: number;
}

export type EvolutionNotificationEventKind =
  | "run_attention" | "automatic_application" | "probation_regression"
  | "breaker_opened" | "breaker_recovered";

export interface EvolutionNotificationEvent {
  schemaVersion: 1;
  eventId: string;
  eventKind: EvolutionNotificationEventKind;
  workspaceId: string;
  runId: string | null;
  applicationId: string | null;
  probationId: string | null;
  breakerId: string | null;
  skillId: string | null;
  safeReasonCode: string | null;
  probationEndsAtMs: number | null;
  entityRevision: number;
  mockProvenance?: "web_simulation";
}

export interface SkillEvolutionOrchestrationService {
  getEvolutionSchedulerOverview(workspaceId: string): Promise<EvolutionSchedulerOverview>;
  getEvolutionPolicy(workspaceId: string): Promise<EvolutionPolicy>;
  updateEvolutionPolicy(input: EvolutionPolicyUpdate): Promise<EvolutionPolicy>;
  listEvolutionRuns(input: EvolutionQuery): Promise<EvolutionPage<EvolutionRunSummary>>;
  getEvolutionRun(runId: string): Promise<EvolutionRunDetail>;
  listEvolutionEligibility(input: EvolutionQuery): Promise<EvolutionPage<EvolutionEligibilitySummary>>;
  listEvolutionApplications(input: EvolutionQuery): Promise<EvolutionPage<EvolutionApplicationSummary>>;
  listEvolutionProbations(input: EvolutionQuery): Promise<EvolutionPage<EvolutionProbationSummary>>;
  listEvolutionBreakers(input: EvolutionQuery): Promise<EvolutionPage<EvolutionBreakerSummary>>;
  requestEvolutionRun(workspaceId: string): Promise<EvolutionRunRequestReceipt>;
  cancelEvolutionRun(runId: string, expectedRevision: number): Promise<EvolutionRunMutationReceipt>;
  acknowledgeEvolutionBreaker(
    breakerId: string,
    expectedRevision: number,
  ): Promise<EvolutionBreakerSummary>;
  subscribeEvolutionNotifications(
    handler: (event: EvolutionNotificationEvent) => void,
  ): Promise<() => void>;
}

declare module "./agent-service" {
  interface AgentService {
    getEvolutionSchedulerOverview: SkillEvolutionOrchestrationService["getEvolutionSchedulerOverview"];
    getEvolutionPolicy: SkillEvolutionOrchestrationService["getEvolutionPolicy"];
    updateEvolutionPolicy: SkillEvolutionOrchestrationService["updateEvolutionPolicy"];
    listEvolutionRuns: SkillEvolutionOrchestrationService["listEvolutionRuns"];
    getEvolutionRun: SkillEvolutionOrchestrationService["getEvolutionRun"];
    listEvolutionEligibility: SkillEvolutionOrchestrationService["listEvolutionEligibility"];
    listEvolutionApplications: SkillEvolutionOrchestrationService["listEvolutionApplications"];
    listEvolutionProbations: SkillEvolutionOrchestrationService["listEvolutionProbations"];
    listEvolutionBreakers: SkillEvolutionOrchestrationService["listEvolutionBreakers"];
    requestEvolutionRun: SkillEvolutionOrchestrationService["requestEvolutionRun"];
    cancelEvolutionRun: SkillEvolutionOrchestrationService["cancelEvolutionRun"];
    acknowledgeEvolutionBreaker: SkillEvolutionOrchestrationService["acknowledgeEvolutionBreaker"];
    subscribeEvolutionNotifications: SkillEvolutionOrchestrationService["subscribeEvolutionNotifications"];
  }
}
