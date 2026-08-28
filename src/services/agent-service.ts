import type { Session } from "../types/agent";

export interface EvidenceQueryInput {
  workspace?: string;
  skillId?: string;
  limit?: number;
  cursor?: string;
}

export interface EvidenceSignalSummary {
  signalId: string;
  sourceKind: string;
  category: string;
  polarity: string;
  severity: string;
  attribution: string;
  attributionRationale: string;
  sourceFidelity: string;
  sourceAgentId?: string;
  extractorId: string;
  extractorVersion: number;
  safeSummary: string;
  occurredAt: string;
  sanitizerVersion: number;
  associationTruncatedCount: number;
  sourceLinkTruncatedCount: number;
}

export interface EvidenceSeedSummary {
  seedId: string;
  category: string;
  readiness: string;
  readinessReason: string;
  safeSummary: string;
  signalCount: number;
  truncatedSignalCount: number;
  independentRunCount: number;
  hasRecovery: boolean;
  firstOccurredAt: string;
  lastOccurredAt: string;
  createdAt: string;
}

export interface EvidencePipelineSummary {
  collectionEnabled: boolean;
  status: "healthy" | "degraded" | "disabled";
  queueDepth: number;
  failureCount: number;
}

export interface EvidenceOverview {
  signalCount: number;
  seedCount: number;
  firstOccurredAt?: string;
  lastOccurredAt?: string;
  distributions: Record<string, Record<string, number>>;
  signals: EvidenceSignalSummary[];
  seeds: EvidenceSeedSummary[];
  nextCursor?: string;
  pipeline: EvidencePipelineSummary;
  retentionDays: number;
  signalQuota: number;
  seedQuota: number;
  byteQuota: number;
  droppedCount: number;
  expiredCount: number;
}

export interface EvidenceSeedLineage {
  seed: EvidenceSeedSummary;
  signals: EvidenceSignalSummary[];
}

export interface PurgeEvidenceInput {
  operationId: string;
  workspace?: string;
  skillId: string;
  confirmed: boolean;
}

export interface PurgeEvidenceOutcome {
  operationId: string;
  deletedSignals: number;
  deletedSeeds: number;
  deletedFeedback: number;
}
import type { DesktopUpdateSnapshot, UpdateOperationReceipt, UpdatePreferences } from "../types/desktop-update";
import type { EvaluationService } from "./evaluation-service";
import type {
  ApiAgentService,
  HybridRoutingService,
  OnePieceProfileService,
  OnePieceProviderService,
} from "./api-provider-service";
import type {
  SessionLogExportResult,
  SessionLogEntry,
  SessionLogPage,
  SessionLogQuery,
} from "../types/session-workspace";
import type { SessionWorkspaceInspectionService } from "./session-workspace-inspection-service";
import type { SkillBindingService, SkillCatalogService, SkillOverlayService } from "./skill-service";
import type { PromptHookService } from "./prompt-hook-service";
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences, OpenSessionFolderResult, SaveFolderOpenerPreferencesInput } from "../types/folder-opener";
import type {
  ApplyCliConfigProfileInput,
  CliConfigApplyResult,
} from "../types/cli-agent-config";
import type {
  ExpertRoleService,
  KnownWorkspaceService,
  SessionCategoryService,
} from "./session-organization-service";
import type { AgentTerminalService } from "./agent-terminal-service";
import type { MissionControlService } from "./mission-control-service";
import type { LoopService } from "./loop-service";
import type { UsageStatisticsService } from "./usage-statistics-service";
import type { LspService } from "./lsp-service";
import type { BuiltinToolService } from "./builtin-tool-service";
import type { CliConfigService, CliParameterService, CliToolService } from "./cli-service";
import type { AgentRegistryService } from "./agent-registry-service";
import type { AgentMemoryService } from "./agent-memory-service";
import type { ChatMessagingService } from "./chat-messaging-service";
import type { SessionChatConfigService } from "./session-chat-config-service";
import type { SessionLifecycleService, SessionSeatService } from "./session-lifecycle-service";
import type { SessionQueryService } from "./session-query-service";
import type { SessionRecoveryService } from "./session-recovery-service";
import type { CodeIndexService } from "./code-index-service";
import type { SkillEvidenceService, SkillGovernanceService } from "./skill-governance-service";
import type { ContextQualityService, ScheduledTaskService } from "./scheduled-task-service";
import type { CodeReviewService } from "./code-review-service";
import type { SessionWorkspaceEvidenceService } from "./session-workspace-evidence-service";

export interface AgentService extends
  ApiAgentService,
  BuiltinToolService,
  CodeReviewService,
  SessionWorkspaceEvidenceService,
  CliConfigService,
  CliParameterService,
  CliToolService,
  CodeIndexService,
  EvaluationService,
  HybridRoutingService,
  OnePieceProfileService,
  OnePieceProviderService,
  PromptHookService,
  ScheduledTaskService,
  ContextQualityService,
  AgentRegistryService,
  SkillGovernanceService,
  SkillEvidenceService,
  SkillCatalogService,
  SkillBindingService,
  SkillOverlayService,
  SessionCategoryService,
  ExpertRoleService,
  KnownWorkspaceService,
  AgentTerminalService,
  UsageStatisticsService,
  MissionControlService,
  AgentMemoryService,
  ChatMessagingService,
  SessionChatConfigService,
  SessionLifecycleService,
  SessionQueryService,
  SessionRecoveryService,
  SessionSeatService,
  LspService,
  SessionWorkspaceInspectionService,
  LoopService {
  getDesktopUpdateSnapshot(): Promise<DesktopUpdateSnapshot>;
  getDesktopUpdatePreferences(): Promise<UpdatePreferences>;
  saveDesktopUpdatePreferences(input: UpdatePreferences): Promise<UpdatePreferences>;
  checkForDesktopUpdate(): Promise<UpdateOperationReceipt>;
  downloadAndInstallDesktopUpdate(): Promise<UpdateOperationReceipt>;
  restartAfterDesktopUpdate(): Promise<void>;
  deleteApiAgent(agentId: string): Promise<void>;
  applyCliConfigProfile(input: ApplyCliConfigProfileInput): Promise<CliConfigApplyResult>;
  listSessionLogs(input: SessionLogQuery): Promise<SessionLogPage>;
  /** One row by id, which is how a live notice becomes a row without the event carrying one. */
  getSessionLogRecord(recordId: string): Promise<SessionLogEntry | null>;
  exportSessionLogs(input: SessionLogQuery): Promise<SessionLogExportResult>;
  listFolderOpeners(): Promise<FolderOpenerAvailability[]>;
  refreshFolderOpeners(): Promise<FolderOpenerAvailability[]>;
  getFolderOpenerPreferences(): Promise<FolderOpenerPreferences>;
  saveFolderOpenerPreferences(input: SaveFolderOpenerPreferencesInput): Promise<FolderOpenerPreferences>;
  /**
   * Reveals a session's workspace in an external tool.
   *
   * `relativePath` narrows it to a subdirectory. Optional rather than required so every existing
   * caller keeps meaning "the workspace root", and resolved against the canonical root on the
   * native side — a file manager opens whatever absolute path it is handed, so that is the last
   * place a path assembled from a stale tree row can be checked.
   */
  openSessionFolder(
    sessionId: string,
    openerId: FolderOpenerId,
    relativePath?: string,
  ): Promise<OpenSessionFolderResult>;
  subscribeFolderOpenerEvents(handler: () => void): Promise<() => void>;
  subscribeSessionEvents(handler: (event: SessionStateEvent) => void): Promise<() => void>;
}

export type SessionStateEvent =
  | { kind: "active-session-changed"; sessionId: string | null }
  | { kind: "configuration-changed"; sessionId: string }
  | RecoverySessionStateEvent;

export type RecoveryDecision =
  | "completed"
  | "failed"
  | "cancelled"
  | "interrupted_without_tool_ambiguity"
  | "action_required"
  | "quarantined"
  | "retry_later"
  | "acknowledged";

export type RecoveryTrigger = "startup" | "explicit_retry" | "user_acknowledgement";

export type RecoveryReasonCode =
  | "confirmed_completed_message"
  | "confirmed_failed_message"
  | "confirmed_cancelled_operation"
  | "interrupted_tool_free_response"
  | "missing_execution_run"
  | "missing_assistant_message"
  | "unfinished_tool_activity"
  | "opaque_provider_activity"
  | "conflicting_execution_runs"
  | "conflicting_terminal_outcomes"
  | "invalid_message_sequence"
  | "invalid_execution_correlation"
  | "live_runtime_handle"
  | "storage_temporarily_unavailable"
  | "acknowledged_by_user";

export type RecoveryEvidenceReference =
  | { kind: "session"; sessionId: string; stateRevision: number; historyRevision: number }
  | { kind: "message"; messageId: string; executionRunId: string | null; status: string }
  | { kind: "operation"; operationId: string; executionRunId: string | null; status: string }
  | { kind: "tool_activity"; toolUseId: string; executionRunId: string | null; status: string }
  | { kind: "provider_resume_metadata"; present: boolean }
  | { kind: "live_runtime_handle"; executionRunId: string | null; present: boolean };

export interface SessionRecoveryReport {
  reportId: string;
  sessionId: string;
  recoveryRevision: number;
  trigger: RecoveryTrigger;
  observedLifecycle: string;
  observedExecutionRunId: string | null;
  decision: RecoveryDecision;
  reasonCodes: RecoveryReasonCode[];
  evidenceRefs: RecoveryEvidenceReference[];
  createdAt: string;
}

export interface SessionRecoverySummary {
  session: Session;
  latestReport: SessionRecoveryReport | null;
}

export interface SessionRecoveryAcknowledgement {
  session: Session;
  report: SessionRecoveryReport;
}

export type RecoverySessionStateEvent = {
  kind:
    | "recovery-started"
    | "recovery-completed"
    | "recovery-action-required"
    | "recovery-quarantined"
    | "recovery-acknowledged";
  sessionId: string;
  recoveryRevision: number;
};
