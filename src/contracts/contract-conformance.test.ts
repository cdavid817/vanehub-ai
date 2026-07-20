import { describe, expect, it } from "vitest";
import type * as AgentContracts from "./agent";
import type * as ChatContracts from "./chat";
import type * as McpContracts from "./mcp";
import type * as SdkContracts from "./sdk";
import type * as SkillContracts from "./skill";
import type * as OperationContracts from "./operation";
import type * as AgentTypes from "../types/agent";
import type * as ChatTypes from "../types/chat";
import type * as McpTypes from "../types/mcp";
import type * as SdkTypes from "../types/sdk";
import type * as SkillTypes from "../types/skill";
import type * as OperationTypes from "../types/operation";
import type * as SessionWorkspaceContracts from "./session-workspace";
import type * as SessionWorkspaceTypes from "../types/session-workspace";

type Equal<Left, Right> =
  (<Value>() => Value extends Left ? 1 : 2) extends
  (<Value>() => Value extends Right ? 1 : 2)
    ? (<Value>() => Value extends Right ? 1 : 2) extends
      (<Value>() => Value extends Left ? 1 : 2)
      ? true
      : false
    : false;

type Assert<T extends true> = T;

type AgentAssertions = [
  Assert<Equal<AgentContracts.InteractionMode, AgentTypes.InteractionMode>>,
  Assert<Equal<AgentContracts.AvailabilityState, AgentTypes.AvailabilityState>>,
  Assert<Equal<AgentContracts.SessionLifecycleState, AgentTypes.SessionLifecycleState>>,
  Assert<Equal<AgentContracts.LaunchMetadata, AgentTypes.LaunchMetadata>>,
  Assert<Equal<AgentContracts.AgentRegistryEntry, AgentTypes.AgentRegistryEntry>>,
  Assert<Equal<AgentContracts.WorkflowState, AgentTypes.WorkflowState>>,
  Assert<Equal<AgentContracts.Session, AgentTypes.Session>>,
  Assert<Equal<AgentContracts.KnownProject, AgentTypes.KnownProject>>,
  Assert<Equal<AgentContracts.ProjectInspection, AgentTypes.ProjectInspection>>,
  Assert<Equal<AgentContracts.CreateSessionInput, AgentTypes.CreateSessionInput>>,
  Assert<Equal<AgentContracts.ReadinessStatus, AgentTypes.ReadinessStatus>>,
  Assert<Equal<AgentContracts.LaunchResult, AgentTypes.LaunchResult>>,
  Assert<Equal<AgentContracts.SessionDetails, AgentTypes.SessionDetails>>,
  Assert<Equal<AgentContracts.CliVersionCheckStatus, AgentTypes.CliVersionCheckStatus>>,
  Assert<Equal<AgentContracts.CliEnvironmentType, AgentTypes.CliEnvironmentType>>,
  Assert<Equal<AgentContracts.CliInstallSource, AgentTypes.CliInstallSource>>,
  Assert<Equal<AgentContracts.CliConflictState, AgentTypes.CliConflictState>>,
  Assert<Equal<AgentContracts.CliLifecycleEligibility, AgentTypes.CliLifecycleEligibility>>,
  Assert<Equal<AgentContracts.CliInstallation, AgentTypes.CliInstallation>>,
  Assert<Equal<AgentContracts.CliToolStatus, AgentTypes.CliToolStatus>>,
  Assert<Equal<AgentContracts.CliPackageOperationInput, AgentTypes.CliPackageOperationInput>>,
  Assert<Equal<AgentContracts.ManagedCliAgentId, AgentTypes.ManagedCliAgentId>>,
  Assert<Equal<AgentContracts.CliParameterControl, AgentTypes.CliParameterControl>>,
  Assert<Equal<AgentContracts.CliParameterValue, AgentTypes.CliParameterValue>>,
  Assert<Equal<AgentContracts.CliParameterLaunchScope, AgentTypes.CliParameterLaunchScope>>,
  Assert<Equal<AgentContracts.CliParameterRisk, AgentTypes.CliParameterRisk>>,
  Assert<Equal<AgentContracts.CliParameterOption, AgentTypes.CliParameterOption>>,
  Assert<Equal<AgentContracts.CliParameterDefinition, AgentTypes.CliParameterDefinition>>,
  Assert<Equal<AgentContracts.CliParameterSelections, AgentTypes.CliParameterSelections>>,
  Assert<Equal<AgentContracts.CliParameterProfile, AgentTypes.CliParameterProfile>>,
  Assert<Equal<AgentContracts.SaveCliParameterProfileInput, AgentTypes.SaveCliParameterProfileInput>>,
];

type ChatAssertions = [
  Assert<Equal<ChatContracts.MessageRole, ChatTypes.MessageRole>>,
  Assert<Equal<ChatContracts.MessageStatus, ChatTypes.MessageStatus>>,
  Assert<Equal<ChatContracts.ReasoningDepth, ChatTypes.ReasoningDepth>>,
  Assert<Equal<ChatContracts.PermissionMode, ChatTypes.PermissionMode>>,
  Assert<Equal<ChatContracts.ModelInfo, ChatTypes.ModelInfo>>,
  Assert<Equal<ChatContracts.ChatConfig, ChatTypes.ChatConfig>>,
  Assert<Equal<ChatContracts.ToolUseBlock, ChatTypes.ToolUseBlock>>,
  Assert<Equal<ChatContracts.RichBlockKind, ChatTypes.RichBlockKind>>,
  Assert<Equal<ChatContracts.RichCardBlock, ChatTypes.RichCardBlock>>,
  Assert<Equal<ChatContracts.RichDiffBlock, ChatTypes.RichDiffBlock>>,
  Assert<Equal<ChatContracts.RichChecklistBlock, ChatTypes.RichChecklistBlock>>,
  Assert<Equal<ChatContracts.RichMediaGalleryBlock, ChatTypes.RichMediaGalleryBlock>>,
  Assert<Equal<ChatContracts.RichAudioBlock, ChatTypes.RichAudioBlock>>,
  Assert<Equal<ChatContracts.RichInteractiveOption, ChatTypes.RichInteractiveOption>>,
  Assert<Equal<ChatContracts.RichInteractiveBlock, ChatTypes.RichInteractiveBlock>>,
  Assert<Equal<ChatContracts.RichHtmlWidgetBlock, ChatTypes.RichHtmlWidgetBlock>>,
  Assert<Equal<ChatContracts.RichFileBlock, ChatTypes.RichFileBlock>>,
  Assert<Equal<ChatContracts.RichBlock, ChatTypes.RichBlock>>,
  Assert<Equal<ChatContracts.TokenUsage, ChatTypes.TokenUsage>>,
  Assert<Equal<ChatContracts.ChatMessage, ChatTypes.ChatMessage>>,
  Assert<Equal<ChatContracts.ChatStreamEvent, ChatTypes.ChatStreamEvent>>,
  Assert<Equal<ChatContracts.SendMessageInput, ChatTypes.SendMessageInput>>,
  Assert<Equal<ChatContracts.UsageStatisticsRange, ChatTypes.UsageStatisticsRange>>,
  Assert<Equal<ChatContracts.UsageStatistics, ChatTypes.UsageStatistics>>,
  Assert<Equal<ChatContracts.SessionUsageSummary, ChatTypes.SessionUsageSummary>>,
];

type McpAssertions = [
  Assert<Equal<McpContracts.McpTransportType, McpTypes.McpTransportType>>,
  Assert<Equal<McpContracts.McpConnectionStatus, McpTypes.McpConnectionStatus>>,
  Assert<Equal<McpContracts.McpScope, McpTypes.McpScope>>,
  Assert<Equal<McpContracts.McpServerConfig, McpTypes.McpServerConfig>>,
  Assert<Equal<McpContracts.PartialMcpServerConfig, McpTypes.PartialMcpServerConfig>>,
  Assert<Equal<McpContracts.McpToolInfo, McpTypes.McpToolInfo>>,
  Assert<Equal<McpContracts.McpServerStatus, McpTypes.McpServerStatus>>,
  Assert<Equal<McpContracts.McpTestResult, McpTypes.McpTestResult>>,
  Assert<Equal<McpContracts.McpImportResult, McpTypes.McpImportResult>>,
  Assert<Equal<McpContracts.McpImportServerEntry, McpTypes.McpImportServerEntry>>,
  Assert<Equal<McpContracts.McpImportExport, McpTypes.McpImportExport>>,
];

type SdkAssertions = [
  Assert<Equal<SdkContracts.SdkId, SdkTypes.SdkId>>,
  Assert<Equal<SdkContracts.SdkInstallStatus, SdkTypes.SdkInstallStatus>>,
  Assert<Equal<SdkContracts.SdkVersionSource, SdkTypes.SdkVersionSource>>,
  Assert<Equal<SdkContracts.SdkOperationType, SdkTypes.SdkOperationType>>,
  Assert<Equal<SdkContracts.SdkDefinition, SdkTypes.SdkDefinition>>,
  Assert<Equal<SdkContracts.SdkStatus, SdkTypes.SdkStatus>>,
  Assert<Equal<SdkContracts.SdkVersionInfo, SdkTypes.SdkVersionInfo>>,
  Assert<Equal<SdkContracts.SdkEnvironmentStatus, SdkTypes.SdkEnvironmentStatus>>,
  Assert<Equal<SdkContracts.SdkOperationLog, SdkTypes.SdkOperationLog>>,
  Assert<Equal<SdkContracts.SdkOperationRequest, SdkTypes.SdkOperationRequest>>,
  Assert<Equal<SdkContracts.SdkOperationResult, SdkTypes.SdkOperationResult>>,
  Assert<Equal<SdkContracts.SdkStatusMap, SdkTypes.SdkStatusMap>>,
  Assert<Equal<SdkContracts.SdkVersionMap, SdkTypes.SdkVersionMap>>,
  Assert<Equal<SdkContracts.SdkUpdateMap, SdkTypes.SdkUpdateMap>>,
  Assert<Equal<SdkContracts.SdkVersionAction, SdkTypes.SdkVersionAction>>,
];

type OperationAssertions = [
  Assert<Equal<OperationContracts.OperationKind, OperationTypes.OperationKind>>,
  Assert<Equal<OperationContracts.OperationStatus, OperationTypes.OperationStatus>>,
  Assert<Equal<OperationContracts.OperationLogEntry, OperationTypes.OperationLogEntry>>,
  Assert<Equal<OperationContracts.OperationTask, OperationTypes.OperationTask>>,
];

type SkillAssertions = [
  Assert<Equal<SkillContracts.SkillScope, SkillTypes.SkillScope>>,
  Assert<Equal<SkillContracts.SkillSource, SkillTypes.SkillSource>>,
  Assert<Equal<SkillContracts.SkillScopeInput, SkillTypes.SkillScopeInput>>,
  Assert<Equal<SkillContracts.SkillMetadata, SkillTypes.SkillMetadata>>,
  Assert<Equal<SkillContracts.SkillAgentBinding, SkillTypes.SkillAgentBinding>>,
  Assert<Equal<SkillContracts.Skill, SkillTypes.Skill>>,
  Assert<Equal<SkillContracts.SkillStats, SkillTypes.SkillStats>>,
  Assert<Equal<SkillContracts.SkillListResult, SkillTypes.SkillListResult>>,
  Assert<Equal<SkillContracts.SkillAgentMountPath, SkillTypes.SkillAgentMountPath>>,
  Assert<Equal<SkillContracts.SkillMutationInput, SkillTypes.SkillMutationInput>>,
  Assert<Equal<SkillContracts.SkillUpdateInput, SkillTypes.SkillUpdateInput>>,
  Assert<Equal<SkillContracts.SkillImportInput, SkillTypes.SkillImportInput>>,
  Assert<Equal<SkillContracts.SkillPreview, SkillTypes.SkillPreview>>,
  Assert<Equal<SkillContracts.SkillDriftIssueType, SkillTypes.SkillDriftIssueType>>,
  Assert<Equal<SkillContracts.SkillDriftIssue, SkillTypes.SkillDriftIssue>>,
  Assert<Equal<SkillContracts.SkillDriftReport, SkillTypes.SkillDriftReport>>,
  Assert<Equal<SkillContracts.SkillBackupEntry, SkillTypes.SkillBackupEntry>>,
  Assert<Equal<SkillContracts.SkillSyncResult, SkillTypes.SkillSyncResult>>,
  Assert<Equal<SkillContracts.SkillMountMigrationReport, SkillTypes.SkillMountMigrationReport>>,
];

type SessionWorkspaceAssertions = [
  Assert<Equal<SessionWorkspaceContracts.DirectoryListing, SessionWorkspaceTypes.DirectoryListing>>,
  Assert<Equal<SessionWorkspaceContracts.DocumentListing, SessionWorkspaceTypes.DocumentListing>>,
  Assert<Equal<SessionWorkspaceContracts.FileContent, SessionWorkspaceTypes.FileContent>>,
  Assert<Equal<SessionWorkspaceContracts.GitStatusResult, SessionWorkspaceTypes.GitStatusResult>>,
  Assert<Equal<SessionWorkspaceContracts.GitDiffResult, SessionWorkspaceTypes.GitDiffResult>>,
  Assert<Equal<SessionWorkspaceContracts.SessionLogQuery, SessionWorkspaceTypes.SessionLogQuery>>,
  Assert<Equal<SessionWorkspaceContracts.SessionLogPage, SessionWorkspaceTypes.SessionLogPage>>,
  Assert<Equal<SessionWorkspaceContracts.SessionLogExportResult, SessionWorkspaceTypes.SessionLogExportResult>>,
  Assert<Equal<SessionWorkspaceContracts.ShellSession, SessionWorkspaceTypes.ShellSession>>,
  Assert<Equal<SessionWorkspaceContracts.ShellEvent, SessionWorkspaceTypes.ShellEvent>>,
];

void (0 as unknown as AgentAssertions);
void (0 as unknown as ChatAssertions);
void (0 as unknown as McpAssertions);
void (0 as unknown as SdkAssertions);
void (0 as unknown as OperationAssertions);
void (0 as unknown as SkillAssertions);
void (0 as unknown as SessionWorkspaceAssertions);

describe("contract conformance", () => {
  it("compiles when committed contracts match frontend service types", () => {
    expect(true).toBe(true);
  });
});
