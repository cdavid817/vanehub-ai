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
  CreateShellInput,
  DirectoryListing,
  DocumentListing,
  FileContent,
  FileSearchListing,
  GitDiffResult,
  GitDiffSource,
  GitStatusResult,
  ResizeShellInput,
  SessionLogExportResult,
  SessionLogPage,
  SessionLogQuery,
  ShellEvent,
  ShellSession,
} from "../types/session-workspace";
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
import type { PersonalizationService } from "./personalization-service";
import type { ChatMessagingService } from "./chat-messaging-service";
import type { SessionChatConfigService } from "./session-chat-config-service";
import type { SessionLifecycleService, SessionSeatService } from "./session-lifecycle-service";
import type { SessionQueryService } from "./session-query-service";
import type { SessionRecoveryService } from "./session-recovery-service";
import type { CodeIndexService } from "./code-index-service";
import type { SkillEvidenceService, SkillGovernanceService } from "./skill-governance-service";
import type { ContextQualityService, ScheduledTaskService } from "./scheduled-task-service";
import type { AddReviewCommentInput, CodeReview, ReviewAction, ReviewComment, ReviewDecision, ReviewDiffFile, ReviewRevertReceipt, RevertReviewChangeInput } from "../types/code-review";

export interface AgentService extends
  ApiAgentService,
  BuiltinToolService,
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
  PersonalizationService,
  ChatMessagingService,
  SessionChatConfigService,
  SessionLifecycleService,
  SessionQueryService,
  SessionRecoveryService,
  SessionSeatService,
  LspService,
  LoopService {
  getDesktopUpdateSnapshot(): Promise<DesktopUpdateSnapshot>;
  getDesktopUpdatePreferences(): Promise<UpdatePreferences>;
  saveDesktopUpdatePreferences(input: UpdatePreferences): Promise<UpdatePreferences>;
  checkForDesktopUpdate(): Promise<UpdateOperationReceipt>;
  downloadAndInstallDesktopUpdate(): Promise<UpdateOperationReceipt>;
  restartAfterDesktopUpdate(): Promise<void>;
  openCodeReview(sessionId: string): Promise<CodeReview>;
  getCodeReview(reviewId: string): Promise<CodeReview>;
  loadCodeReviewFile(sessionId: string, path: string, expectedSnapshot: string): Promise<ReviewDiffFile>;
  addCodeReviewComment(input: AddReviewCommentInput): Promise<ReviewComment>;
  resolveCodeReviewComment(reviewId: string, commentId: string): Promise<CodeReview>;
  selectCodeReviewComment(reviewId: string, commentId: string, selected: boolean): Promise<CodeReview>;
  setCodeReviewDecision(reviewId: string, decision: ReviewDecision): Promise<CodeReview>;
  revertCodeReviewChange(input: RevertReviewChangeInput): Promise<ReviewRevertReceipt>;
  sendCodeReviewFeedback(reviewId: string, acknowledgeStale: boolean): Promise<{ messageId: string }>;
  startCodeReviewAction(reviewId: string, action: ReviewAction): Promise<{ operationId: string }>;
  deleteApiAgent(agentId: string): Promise<void>;
  applyCliConfigProfile(input: ApplyCliConfigProfileInput): Promise<CliConfigApplyResult>;
  listSessionDirectory(sessionId: string, path?: string): Promise<DirectoryListing>;
  readSessionFile(sessionId: string, path: string): Promise<FileContent>;
  listSessionDocuments(sessionId: string): Promise<DocumentListing>;
  searchSessionFiles(sessionId: string, query: string, maxResults?: number): Promise<FileSearchListing>;
  getSessionGitStatus(sessionId: string): Promise<GitStatusResult>;
  getSessionGitDiff(sessionId: string, path: string, source: GitDiffSource): Promise<GitDiffResult>;
  listSessionLogs(input: SessionLogQuery): Promise<SessionLogPage>;
  exportSessionLogs(input: SessionLogQuery): Promise<SessionLogExportResult>;
  listFolderOpeners(): Promise<FolderOpenerAvailability[]>;
  refreshFolderOpeners(): Promise<FolderOpenerAvailability[]>;
  getFolderOpenerPreferences(): Promise<FolderOpenerPreferences>;
  saveFolderOpenerPreferences(input: SaveFolderOpenerPreferencesInput): Promise<FolderOpenerPreferences>;
  openSessionFolder(sessionId: string, openerId: FolderOpenerId): Promise<OpenSessionFolderResult>;
  subscribeFolderOpenerEvents(handler: () => void): Promise<() => void>;
  createShell(input: CreateShellInput): Promise<ShellSession>;
  writeShellInput(shellId: string, content: string): Promise<void>;
  resetShellDirectory(shellId: string): Promise<void>;
  resizeShell(input: ResizeShellInput): Promise<void>;
  killShell(shellId: string): Promise<void>;
  subscribeShellEvents(shellId: string, handler: (event: ShellEvent) => void): Promise<() => void>;
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
