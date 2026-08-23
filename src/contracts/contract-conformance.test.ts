import { describe, expect, it } from "vitest";
import type * as AgentContracts from "./agent";
import type * as ChatContracts from "./chat";
import type * as TokenUsageContracts from "./token-usage";
import type * as McpContracts from "./mcp";
import type * as SdkContracts from "./sdk";
import type * as SkillContracts from "./skill";
import type * as SkillConfigurationContracts from "./skill-configuration";
import type * as SkillOverlayContracts from "./skill-overlay";
import type * as SkillOverlayReconciliationContracts from "./skill-overlay-reconciliation";
import type * as OperationContracts from "./operation";
import type * as CliEnvironmentContracts from "./cli-environment";
import type * as CliEnvironmentTypes from "../types/cli-environment";
import type * as CliSnapshotContracts from "./cli-environment-snapshot";
import type * as CliSnapshotTypes from "../types/cli-environment-snapshot";
import type * as OnePieceToolContracts from "./onepiece-tools";
import type * as ObservabilityContracts from "./execution-observability";
import type * as AgentTypes from "../types/agent";
import type * as ChatTypes from "../types/chat";
import type * as TokenUsageTypes from "../types/token-usage";
import type * as McpTypes from "../types/mcp";
import {
  MCP_ERROR_CODES as contractMcpErrorCodes,
  MCP_LIMITS as contractMcpLimits,
} from "./mcp";
import {
  MCP_ERROR_CODES as frontendMcpErrorCodes,
  MCP_LIMITS as frontendMcpLimits,
} from "../types/mcp";
import type * as SdkTypes from "../types/sdk";
import type * as SkillTypes from "../types/skill";
import type * as SkillConfigurationTypes from "../types/skill-configuration";
import type * as SkillOverlayTypes from "../types/skill-overlay";
import type * as SkillOverlayReconciliationTypes from "../types/skill-overlay-reconciliation";
import {
  SKILL_FIXED_READ_TOOLS as contractSkillReadTools,
  SKILL_LEGACY_COMPATIBILITY_DEFAULTS as contractSkillDefaults,
  SKILL_LOGICAL_URI_PREFIX as contractSkillUriPrefix,
  SKILL_MANAGEMENT_SCOPES as contractSkillScopes,
  SKILL_RUNTIME_LAYERS as contractSkillLayers,
} from "./skill";
import {
  SKILL_FIXED_READ_TOOLS as frontendSkillReadTools,
  SKILL_LEGACY_COMPATIBILITY_DEFAULTS as frontendSkillDefaults,
  SKILL_LOGICAL_URI_PREFIX as frontendSkillUriPrefix,
  SKILL_MANAGEMENT_SCOPES as frontendSkillScopes,
  SKILL_RUNTIME_LAYERS as frontendSkillLayers,
} from "../types/skill";
import type * as OperationTypes from "../types/operation";
import type * as OnePieceToolTypes from "../types/onepiece-tools";
import type * as ObservabilityTypes from "../types/execution-observability";
import type * as LoopContracts from "./loop";
import type * as LoopTypes from "../types/loop";
import type * as SessionWorkspaceContracts from "./session-workspace";
import type * as SessionWorkspaceTypes from "../types/session-workspace";
import type * as ContextQualityContracts from "./context-quality";
import type * as ContextQualityTypes from "../types/context-quality";
import type * as MissionControlContracts from "./mission-control";
import type * as MissionControlTypes from "../types/mission-control";
import type * as AgentRunnerContracts from "./agent-runner";
import type * as AgentRunnerTypes from "../types/agent-runner";

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
  Assert<Equal<AgentContracts.ApiInterfaceFormat, AgentTypes.ApiInterfaceFormat>>,
  Assert<Equal<AgentContracts.RegisterApiAgentInput, AgentTypes.RegisterApiAgentInput>>,
  Assert<Equal<AgentContracts.OnePieceProviderConfig, AgentTypes.OnePieceProviderConfig>>,
  Assert<Equal<AgentContracts.SaveOnePieceProviderConfigInput, AgentTypes.SaveOnePieceProviderConfigInput>>,
  Assert<Equal<AgentContracts.OnePieceProviderProfile, AgentTypes.OnePieceProviderProfile>>,
  Assert<Equal<AgentContracts.OnePieceProviderPreset, AgentTypes.OnePieceProviderPreset>>,
  Assert<Equal<AgentContracts.OnePieceProviderProfiles, AgentTypes.OnePieceProviderProfiles>>,
  Assert<Equal<AgentContracts.DiscoverOnePieceProviderModelsInput, AgentTypes.DiscoverOnePieceProviderModelsInput>>,
  Assert<Equal<AgentContracts.OnePieceProviderModelOption, AgentTypes.OnePieceProviderModelOption>>,
  Assert<Equal<AgentContracts.OnePieceProviderModelDiscoveryResult, AgentTypes.OnePieceProviderModelDiscoveryResult>>,
  Assert<Equal<AgentContracts.SaveOnePieceProviderProfileInput, AgentTypes.SaveOnePieceProviderProfileInput>>,
  Assert<Equal<AgentContracts.SaveCustomOnePieceProviderProfileInput, AgentTypes.SaveCustomOnePieceProviderProfileInput>>,
  Assert<Equal<AgentContracts.EndpointProfileMetadata, AgentTypes.EndpointProfileMetadata>>,
  Assert<Equal<AgentContracts.HybridRoutingRule, AgentTypes.HybridRoutingRule>>,
  Assert<Equal<AgentContracts.HybridRoutePreviewInput, AgentTypes.HybridRoutePreviewInput>>,
  Assert<Equal<AgentContracts.HybridRoutePreview, AgentTypes.HybridRoutePreview>>,
  Assert<Equal<AgentContracts.LocalModelDiscoveryResult, AgentTypes.LocalModelDiscoveryResult>>,
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
  Assert<Equal<ChatContracts.SessionExecutionMode, ChatTypes.SessionExecutionMode>>,
  Assert<Equal<ChatContracts.EffectiveExecutionPolicy, ChatTypes.EffectiveExecutionPolicy>>,
  Assert<Equal<ChatContracts.ModelInfo, ChatTypes.ModelInfo>>,
  Assert<Equal<ChatContracts.ChatConfig, ChatTypes.ChatConfig>>,
  Assert<Equal<ChatContracts.ChatFileReference, ChatTypes.ChatFileReference>>,
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

type TokenUsageAssertions = [
  Assert<Equal<TokenUsageContracts.TokenUsageSummaryQuery, TokenUsageTypes.TokenUsageSummaryQuery>>,
  Assert<Equal<TokenUsageContracts.TokenUsageDetailsQuery, TokenUsageTypes.TokenUsageDetailsQuery>>,
  Assert<Equal<TokenUsageContracts.TokenUsageSummary, TokenUsageTypes.TokenUsageSummary>>,
  Assert<Equal<TokenUsageContracts.TokenUsageDetailsPage, TokenUsageTypes.TokenUsageDetailsPage>>,
];

type McpAssertions = [
  Assert<Equal<McpContracts.McpTransportType, McpTypes.McpTransportType>>,
  Assert<Equal<McpContracts.McpConnectionStatus, McpTypes.McpConnectionStatus>>,
  Assert<Equal<McpContracts.McpScope, McpTypes.McpScope>>,
  Assert<Equal<McpContracts.McpImportTransportType, McpTypes.McpImportTransportType>>,
  Assert<Equal<McpContracts.McpErrorCode, McpTypes.McpErrorCode>>,
  Assert<Equal<typeof McpContracts.MCP_ERROR_CODES, typeof McpTypes.MCP_ERROR_CODES>>,
  Assert<Equal<typeof McpContracts.MCP_LIMITS, typeof McpTypes.MCP_LIMITS>>,
  Assert<Equal<McpContracts.McpServerConfig, McpTypes.McpServerConfig>>,
  Assert<Equal<McpContracts.PartialMcpServerConfig, McpTypes.PartialMcpServerConfig>>,
  Assert<Equal<McpContracts.McpToolInfo, McpTypes.McpToolInfo>>,
  Assert<Equal<McpContracts.McpServerStatus, McpTypes.McpServerStatus>>,
  Assert<Equal<McpContracts.McpTestResult, McpTypes.McpTestResult>>,
  Assert<Equal<McpContracts.McpToolCallResult, McpTypes.McpToolCallResult>>,
  Assert<Equal<McpContracts.McpImportResult, McpTypes.McpImportResult>>,
  Assert<Equal<McpContracts.McpImportFailure, McpTypes.McpImportFailure>>,
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

type SkillOverlayAssertions = [
  Assert<Equal<SkillOverlayContracts.SkillOverlayScope, SkillOverlayTypes.SkillOverlayScope>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayStatus, SkillOverlayTypes.SkillOverlayStatus>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayScopeStatus, SkillOverlayTypes.SkillOverlayScopeStatus>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayMutationKind, SkillOverlayTypes.SkillOverlayMutationKind>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayMutationState, SkillOverlayTypes.SkillOverlayMutationState>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayTargetInput, SkillOverlayTypes.SkillOverlayTargetInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayWitnesses, SkillOverlayTypes.SkillOverlayWitnesses>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlaySummary, SkillOverlayTypes.SkillOverlaySummary>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayDetail, SkillOverlayTypes.SkillOverlayDetail>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayDiff, SkillOverlayTypes.SkillOverlayDiff>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayScopeDiff, SkillOverlayTypes.SkillOverlayScopeDiff>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayPreviewInput, SkillOverlayTypes.SkillOverlayPreviewInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayPreview, SkillOverlayTypes.SkillOverlayPreview>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayPatchInput, SkillOverlayTypes.SkillOverlayPatchInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayGuidanceInput, SkillOverlayTypes.SkillOverlayGuidanceInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayFileInput, SkillOverlayTypes.SkillOverlayFileInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayMutationStateInput, SkillOverlayTypes.SkillOverlayMutationStateInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayMutationOutcome, SkillOverlayTypes.SkillOverlayMutationOutcome>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayImportInput, SkillOverlayTypes.SkillOverlayImportInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayImportReview, SkillOverlayTypes.SkillOverlayImportReview>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayPromotionInput, SkillOverlayTypes.SkillOverlayPromotionInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayHistoryInput, SkillOverlayTypes.SkillOverlayHistoryInput>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayHistoryPage, SkillOverlayTypes.SkillOverlayHistoryPage>>,
  Assert<Equal<SkillOverlayContracts.SkillOverlayScanResult, SkillOverlayTypes.SkillOverlayScanResult>>,
  Assert<
    Equal<
      SkillOverlayReconciliationContracts.SkillOverlayReconciliationInput,
      SkillOverlayReconciliationTypes.SkillOverlayReconciliationInput
    >
  >,
  Assert<
    Equal<
      SkillOverlayReconciliationContracts.SkillOverlayReconciliationPreview,
      SkillOverlayReconciliationTypes.SkillOverlayReconciliationPreview
    >
  >,
  Assert<
    Equal<
      SkillOverlayReconciliationContracts.SkillOverlayServiceError,
      SkillOverlayReconciliationTypes.SkillOverlayServiceError
    >
  >,
];

type OperationAssertions = [
  Assert<Equal<OperationContracts.OperationKind, OperationTypes.OperationKind>>,
  Assert<Equal<OperationContracts.OperationStatus, OperationTypes.OperationStatus>>,
  Assert<Equal<OperationContracts.OperationLogEntry, OperationTypes.OperationLogEntry>>,
  Assert<Equal<OperationContracts.OperationTask, OperationTypes.OperationTask>>,
];

type CliEnvironmentAssertions = [
  Assert<Equal<CliEnvironmentContracts.CliMutationOutcome, CliEnvironmentTypes.CliMutationOutcome>>,
  Assert<Equal<CliEnvironmentContracts.CliPlanRejectionCode, CliEnvironmentTypes.CliPlanRejectionCode>>,
  Assert<Equal<CliEnvironmentContracts.CliMutationResult, CliEnvironmentTypes.CliMutationResult>>,
  Assert<Equal<CliEnvironmentContracts.CliOperationPhase, CliEnvironmentTypes.CliOperationPhase>>,
  Assert<
    Equal<
      CliEnvironmentContracts.CliOperationTermination,
      CliEnvironmentTypes.CliOperationTermination
    >
  >,
  Assert<
    Equal<
      CliEnvironmentContracts.CliVerificationWarning,
      CliEnvironmentTypes.CliVerificationWarning
    >
  >,
];

type CliSnapshotAssertions = [
  Assert<Equal<CliSnapshotContracts.CliInstallation, CliSnapshotTypes.CliInstallation>>,
  Assert<Equal<CliSnapshotContracts.CliConflict, CliSnapshotTypes.CliConflict>>,
  Assert<Equal<CliSnapshotContracts.CliSourceSummary, CliSnapshotTypes.CliSourceSummary>>,
  Assert<Equal<CliSnapshotContracts.CliSourceCapabilities, CliSnapshotTypes.CliSourceCapabilities>>,
  Assert<Equal<CliSnapshotContracts.CliAllowedAction, CliSnapshotTypes.CliAllowedAction>>,
  Assert<Equal<CliSnapshotContracts.CliMutationSummary, CliSnapshotTypes.CliMutationSummary>>,
  Assert<Equal<CliSnapshotContracts.CliEnvironmentSnapshot, CliSnapshotTypes.CliEnvironmentSnapshot>>,
  Assert<Equal<CliSnapshotContracts.CliCommandPreview, CliSnapshotTypes.CliCommandPreview>>,
  Assert<Equal<CliSnapshotContracts.CliActionPlan, CliSnapshotTypes.CliActionPlan>>,
  Assert<Equal<CliSnapshotContracts.CliBulkActionItem, CliSnapshotTypes.CliBulkActionItem>>,
  Assert<Equal<CliSnapshotContracts.CliBulkSkip, CliSnapshotTypes.CliBulkSkip>>,
  Assert<Equal<CliSnapshotContracts.CliBulkActionPlan, CliSnapshotTypes.CliBulkActionPlan>>,
  Assert<Equal<CliSnapshotContracts.PrepareCliActionInput, CliSnapshotTypes.PrepareCliActionInput>>,
  Assert<Equal<CliSnapshotContracts.ExecuteCliActionInput, CliSnapshotTypes.ExecuteCliActionInput>>,
];

type OnePieceToolAssertions = [
  Assert<Equal<OnePieceToolContracts.OnePieceToolCapability, OnePieceToolTypes.OnePieceToolCapability>>,
  Assert<Equal<OnePieceToolContracts.OnePieceToolReadiness, OnePieceToolTypes.OnePieceToolReadiness>>,
  Assert<Equal<OnePieceToolContracts.OnePieceToolOperation, OnePieceToolTypes.OnePieceToolOperation>>,
  Assert<Equal<OnePieceToolContracts.OnePieceToolApproval, OnePieceToolTypes.OnePieceToolApproval>>,
  Assert<Equal<OnePieceToolContracts.ArtifactSummary, OnePieceToolTypes.ArtifactSummary>>,
  Assert<Equal<OnePieceToolContracts.ArtifactDetail, OnePieceToolTypes.ArtifactDetail>>,
  Assert<Equal<OnePieceToolContracts.DelegationAttempt, OnePieceToolTypes.DelegationAttempt>>,
  Assert<Equal<OnePieceToolContracts.DelegationView, OnePieceToolTypes.DelegationView>>,
  Assert<Equal<OnePieceToolContracts.ChangeSetView, OnePieceToolTypes.ChangeSetView>>,
  Assert<Equal<OnePieceToolContracts.ChangeSetApplyAttempt, OnePieceToolTypes.ChangeSetApplyAttempt>>,
  Assert<Equal<OnePieceToolContracts.ChangeSetRecoveryState, OnePieceToolTypes.ChangeSetRecoveryState>>,
  Assert<Equal<OnePieceToolContracts.BrowserHumanHandoff, OnePieceToolTypes.BrowserHumanHandoff>>,
];

type ObservabilityAssertions = [
  Assert<Equal<ObservabilityContracts.CapturePolicy, ObservabilityTypes.CapturePolicy>>,
  Assert<Equal<ObservabilityContracts.OtlpProtocol, ObservabilityTypes.OtlpProtocol>>,
  Assert<Equal<ObservabilityContracts.ObservabilitySettings, ObservabilityTypes.ObservabilitySettings>>,
  Assert<Equal<ObservabilityContracts.ExecutionStatus, ObservabilityTypes.ExecutionStatus>>,
  Assert<Equal<ObservabilityContracts.ExecutionFidelity, ObservabilityTypes.ExecutionFidelity>>,
  Assert<Equal<ObservabilityContracts.ExecutionSource, ObservabilityTypes.ExecutionSource>>,
  Assert<Equal<ObservabilityContracts.SafeAttribute, ObservabilityTypes.SafeAttribute>>,
  Assert<Equal<ObservabilityContracts.ExecutionRunSummary, ObservabilityTypes.ExecutionRunSummary>>,
  Assert<Equal<ObservabilityContracts.ExecutionSpanSummary, ObservabilityTypes.ExecutionSpanSummary>>,
  Assert<Equal<ObservabilityContracts.ExecutionEvent, ObservabilityTypes.ExecutionEvent>>,
  Assert<Equal<ObservabilityContracts.ExecutionTimeline, ObservabilityTypes.ExecutionTimeline>>,
  Assert<Equal<ObservabilityContracts.PageRequest, ObservabilityTypes.PageRequest>>,
  Assert<Equal<ObservabilityContracts.ExecutionRunPage, ObservabilityTypes.ExecutionRunPage>>,
  Assert<Equal<ObservabilityContracts.ObservabilityErrorCode, ObservabilityTypes.ObservabilityErrorCode>>,
  Assert<Equal<ObservabilityContracts.ObservabilityCommandError, ObservabilityTypes.ObservabilityCommandError>>,
];

type LoopAssertions = [
  Assert<Equal<LoopContracts.LoopRunStatus, LoopTypes.LoopRunStatus>>,
  Assert<Equal<LoopContracts.LoopRunPhase, LoopTypes.LoopRunPhase>>,
  Assert<Equal<LoopContracts.LoopTerminalReason, LoopTypes.LoopTerminalReason>>,
  Assert<Equal<LoopContracts.LoopRole, LoopTypes.LoopRole>>,
  Assert<Equal<LoopContracts.LoopVerifierRecommendation, LoopTypes.LoopVerifierRecommendation>>,
  Assert<Equal<LoopContracts.LoopEvidenceKind, LoopTypes.LoopEvidenceKind>>,
  Assert<Equal<LoopContracts.LoopEvidenceStatus, LoopTypes.LoopEvidenceStatus>>,
  Assert<Equal<LoopContracts.LoopVerificationCommand, LoopTypes.LoopVerificationCommand>>,
  Assert<Equal<LoopContracts.LoopLimits, LoopTypes.LoopLimits>>,
  Assert<Equal<LoopContracts.LoopDefinition, LoopTypes.LoopDefinition>>,
  Assert<Equal<LoopContracts.SaveLoopDefinitionInput, LoopTypes.SaveLoopDefinitionInput>>,
  Assert<Equal<LoopContracts.LoopEvidence, LoopTypes.LoopEvidence>>,
  Assert<Equal<LoopContracts.LoopIteration, LoopTypes.LoopIteration>>,
  Assert<Equal<LoopContracts.LoopRun, LoopTypes.LoopRun>>,
  Assert<Equal<LoopContracts.StartLoopResult, LoopTypes.StartLoopResult>>,
  Assert<Equal<LoopContracts.LoopEventKind, LoopTypes.LoopEventKind>>,
  Assert<Equal<LoopContracts.LoopEvent, LoopTypes.LoopEvent>>,
  Assert<Equal<LoopContracts.ContinueLoopInput, LoopTypes.ContinueLoopInput>>,
];

type SkillAssertions = [
  Assert<Equal<typeof contractSkillScopes, typeof frontendSkillScopes>>,
  Assert<Equal<typeof contractSkillLayers, typeof frontendSkillLayers>>,
  Assert<Equal<typeof contractSkillReadTools, typeof frontendSkillReadTools>>,
  Assert<Equal<typeof contractSkillUriPrefix, typeof frontendSkillUriPrefix>>,
  Assert<Equal<typeof contractSkillDefaults, typeof frontendSkillDefaults>>,
  Assert<Equal<SkillContracts.SkillScope, SkillTypes.SkillScope>>,
  Assert<Equal<SkillContracts.SkillSource, SkillTypes.SkillSource>>,
  Assert<Equal<SkillContracts.SkillType, SkillTypes.SkillType>>,
  Assert<Equal<SkillContracts.SkillDelivery, SkillTypes.SkillDelivery>>,
  Assert<Equal<SkillContracts.SkillLayer, SkillTypes.SkillLayer>>,
  Assert<Equal<SkillContracts.SkillOrigin, SkillTypes.SkillOrigin>>,
  Assert<Equal<SkillContracts.SkillTrust, SkillTypes.SkillTrust>>,
  Assert<Equal<SkillContracts.SkillAvailability, SkillTypes.SkillAvailability>>,
  Assert<Equal<SkillContracts.SkillDelegationCapability, SkillTypes.SkillDelegationCapability>>,
  Assert<Equal<SkillContracts.SkillScopeInput, SkillTypes.SkillScopeInput>>,
  Assert<Equal<SkillContracts.SkillMetadata, SkillTypes.SkillMetadata>>,
  Assert<Equal<SkillContracts.SkillCompatibilityDefaults, SkillTypes.SkillCompatibilityDefaults>>,
  Assert<Equal<SkillContracts.SkillShadowSummary, SkillTypes.SkillShadowSummary>>,
  Assert<Equal<SkillContracts.SkillUsageSummary, SkillTypes.SkillUsageSummary>>,
  Assert<Equal<SkillContracts.SkillAgentBinding, SkillTypes.SkillAgentBinding>>,
  Assert<Equal<SkillContracts.Skill, SkillTypes.Skill>>,
  Assert<Equal<SkillContracts.SkillStats, SkillTypes.SkillStats>>,
  Assert<Equal<SkillContracts.SkillListResult, SkillTypes.SkillListResult>>,
  Assert<Equal<SkillContracts.SkillAgentMountPath, SkillTypes.SkillAgentMountPath>>,
  Assert<Equal<SkillContracts.SkillMutationInput, SkillTypes.SkillMutationInput>>,
  Assert<Equal<SkillContracts.SkillUpdateInput, SkillTypes.SkillUpdateInput>>,
  Assert<Equal<SkillContracts.SkillImportInput, SkillTypes.SkillImportInput>>,
  Assert<Equal<SkillContracts.SkillPreview, SkillTypes.SkillPreview>>,
  Assert<Equal<SkillContracts.SkillLoadInput, SkillTypes.SkillLoadInput>>,
  Assert<Equal<SkillContracts.SkillResourceReadInput, SkillTypes.SkillResourceReadInput>>,
  Assert<Equal<SkillContracts.SkillResourceEntry, SkillTypes.SkillResourceEntry>>,
  Assert<Equal<SkillContracts.SkillResourceIndex, SkillTypes.SkillResourceIndex>>,
  Assert<Equal<SkillContracts.SkillLoadResult, SkillTypes.SkillLoadResult>>,
  Assert<Equal<SkillContracts.SkillResourceReadResult, SkillTypes.SkillResourceReadResult>>,
  Assert<Equal<SkillContracts.SkillAccessRefusalReason, SkillTypes.SkillAccessRefusalReason>>,
  Assert<Equal<SkillContracts.SkillAccessRefusal, SkillTypes.SkillAccessRefusal>>,
  Assert<Equal<SkillContracts.SkillLoadOutcome, SkillTypes.SkillLoadOutcome>>,
  Assert<Equal<SkillContracts.SkillResourceReadOutcome, SkillTypes.SkillResourceReadOutcome>>,
  Assert<Equal<SkillContracts.SkillDriftIssueType, SkillTypes.SkillDriftIssueType>>,
  Assert<Equal<SkillContracts.SkillDriftIssue, SkillTypes.SkillDriftIssue>>,
  Assert<Equal<SkillContracts.SkillDriftReport, SkillTypes.SkillDriftReport>>,
  Assert<Equal<SkillContracts.SkillBackupEntry, SkillTypes.SkillBackupEntry>>,
  Assert<Equal<SkillContracts.SkillSyncResult, SkillTypes.SkillSyncResult>>,
  Assert<Equal<SkillContracts.SkillMountMigrationReport, SkillTypes.SkillMountMigrationReport>>,
];

type SkillConfigurationAssertions = [
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationScope, SkillConfigurationTypes.SkillConfigurationScope>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationProvenance, SkillConfigurationTypes.SkillConfigurationProvenance>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationSecretState, SkillConfigurationTypes.SkillConfigurationSecretState>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationDrift, SkillConfigurationTypes.SkillConfigurationDrift>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationReadiness, SkillConfigurationTypes.SkillConfigurationReadiness>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurableState, SkillConfigurationTypes.SkillConfigurableState>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationCleanupState, SkillConfigurationTypes.SkillConfigurationCleanupState>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationConsumption, SkillConfigurationTypes.SkillConfigurationConsumption>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationScalarType, SkillConfigurationTypes.SkillConfigurationScalarType>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationFieldKind, SkillConfigurationTypes.SkillConfigurationFieldKind>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationFieldType, SkillConfigurationTypes.SkillConfigurationFieldType>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationValue, SkillConfigurationTypes.SkillConfigurationValue>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationValueEntry, SkillConfigurationTypes.SkillConfigurationValueEntry>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationConstraints, SkillConfigurationTypes.SkillConfigurationConstraints>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationPresentation, SkillConfigurationTypes.SkillConfigurationPresentation>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationGroupDescriptor, SkillConfigurationTypes.SkillConfigurationGroupDescriptor>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationFieldDescriptor, SkillConfigurationTypes.SkillConfigurationFieldDescriptor>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationDescriptor, SkillConfigurationTypes.SkillConfigurationDescriptor>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationProperty, SkillConfigurationTypes.SkillConfigurationProperty>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationScopeState, SkillConfigurationTypes.SkillConfigurationScopeState>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationResolution, SkillConfigurationTypes.SkillConfigurationResolution>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationRecordState, SkillConfigurationTypes.SkillConfigurationRecordState>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationRuntimeSupport, SkillConfigurationTypes.SkillConfigurationRuntimeSupport>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationView, SkillConfigurationTypes.SkillConfigurationView>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationRecovery, SkillConfigurationTypes.SkillConfigurationRecovery>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationRejectionCode, SkillConfigurationTypes.SkillConfigurationRejectionCode>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationRejection, SkillConfigurationTypes.SkillConfigurationRejection>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationSaveResult, SkillConfigurationTypes.SkillConfigurationSaveResult>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationSaveOutcome, SkillConfigurationTypes.SkillConfigurationSaveOutcome>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationResolutionOutcome, SkillConfigurationTypes.SkillConfigurationResolutionOutcome>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationRetentionOutcome, SkillConfigurationTypes.SkillConfigurationRetentionOutcome>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationSecretIntent, SkillConfigurationTypes.SkillConfigurationSecretIntent>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationSecretIntentInput, SkillConfigurationTypes.SkillConfigurationSecretIntentInput>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationWriteInput, SkillConfigurationTypes.SkillConfigurationWriteInput>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationScopeInput, SkillConfigurationTypes.SkillConfigurationScopeInput>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationRetention, SkillConfigurationTypes.SkillConfigurationRetention>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationObsoleteAction, SkillConfigurationTypes.SkillConfigurationObsoleteAction>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationObsoleteFieldChoice, SkillConfigurationTypes.SkillConfigurationObsoleteFieldChoice>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationReconciliationPlan, SkillConfigurationTypes.SkillConfigurationReconciliationPlan>>,
  Assert<Equal<SkillConfigurationContracts.SkillConfigurationReconciliationInput, SkillConfigurationTypes.SkillConfigurationReconciliationInput>>,
  // The Skill contract re-exports the configuration surface, so a name that lands in only one of
  // the two barrels would compile here and diverge for every consumer that imports from "./skill".
  Assert<Equal<SkillContracts.SkillConfigurationView, SkillTypes.SkillConfigurationView>>,
  Assert<Equal<SkillContracts.SkillConfigurationSaveOutcome, SkillTypes.SkillConfigurationSaveOutcome>>,
];

type SessionWorkspaceAssertions = [
  Assert<Equal<SessionWorkspaceContracts.DirectoryListing, SessionWorkspaceTypes.DirectoryListing>>,
  Assert<Equal<SessionWorkspaceContracts.DocumentListing, SessionWorkspaceTypes.DocumentListing>>,
  Assert<Equal<SessionWorkspaceContracts.FileContent, SessionWorkspaceTypes.FileContent>>,
  Assert<Equal<SessionWorkspaceContracts.FileSearchListing, SessionWorkspaceTypes.FileSearchListing>>,
  Assert<Equal<SessionWorkspaceContracts.FileSearchMatch, SessionWorkspaceTypes.FileSearchMatch>>,
  Assert<Equal<SessionWorkspaceContracts.GitStatusResult, SessionWorkspaceTypes.GitStatusResult>>,
  Assert<Equal<SessionWorkspaceContracts.GitDiffResult, SessionWorkspaceTypes.GitDiffResult>>,
  Assert<Equal<SessionWorkspaceContracts.SessionLogQuery, SessionWorkspaceTypes.SessionLogQuery>>,
  Assert<Equal<SessionWorkspaceContracts.SessionLogPage, SessionWorkspaceTypes.SessionLogPage>>,
  Assert<Equal<SessionWorkspaceContracts.SessionLogExportResult, SessionWorkspaceTypes.SessionLogExportResult>>,
  Assert<Equal<SessionWorkspaceContracts.ShellSession, SessionWorkspaceTypes.ShellSession>>,
  Assert<Equal<SessionWorkspaceContracts.ShellEvent, SessionWorkspaceTypes.ShellEvent>>,
];

type ContextQualityAssertions = [
  Assert<Equal<ContextQualityContracts.ContextQualityRangeDays, ContextQualityTypes.ContextQualityRangeDays>>,
  Assert<Equal<ContextQualityContracts.ContextQualityAssessment, ContextQualityTypes.ContextQualityAssessment>>,
  Assert<Equal<ContextQualityContracts.ContextQualityHistoryQuery, ContextQualityTypes.ContextQualityHistoryQuery>>,
  Assert<Equal<ContextQualityContracts.ContextQualityHistoryPage, ContextQualityTypes.ContextQualityHistoryPage>>,
  Assert<Equal<ContextQualityContracts.ContextQualitySummary, ContextQualityTypes.ContextQualitySummary>>,
  Assert<Equal<ContextQualityContracts.ContextQualitySafeError, ContextQualityTypes.ContextQualitySafeError>>,
];

type MissionControlAssertions = [
  Assert<Equal<MissionControlContracts.MissionControlQuery, MissionControlTypes.MissionControlQuery>>,
  Assert<Equal<MissionControlContracts.MissionControlOverview, MissionControlTypes.MissionControlOverview>>,
  Assert<Equal<MissionControlContracts.MissionControlRunDetail, MissionControlTypes.MissionControlRunDetail>>,
  Assert<Equal<MissionControlContracts.MissionControlActionInput, MissionControlTypes.MissionControlActionInput>>,
  Assert<Equal<MissionControlContracts.MissionControlActionReceipt, MissionControlTypes.MissionControlActionReceipt>>,
  Assert<Equal<MissionControlContracts.MissionControlSafeError, MissionControlTypes.MissionControlSafeError>>,
];

type AgentRunnerAssertions = [
  Assert<Equal<AgentRunnerContracts.AgentRunnerKind, AgentRunnerTypes.AgentRunnerKind>>,
  Assert<Equal<AgentRunnerContracts.AgentRunnerRecovery, AgentRunnerTypes.AgentRunnerRecovery>>,
  Assert<Equal<AgentRunnerContracts.AgentRunnerSelection, AgentRunnerTypes.AgentRunnerSelection>>,
  Assert<Equal<AgentRunnerContracts.AgentRunnerCapabilities, AgentRunnerTypes.AgentRunnerCapabilities>>,
  Assert<Equal<AgentRunnerContracts.AgentRunnerDescriptor, AgentRunnerTypes.AgentRunnerDescriptor>>,
  Assert<Equal<AgentRunnerContracts.AgentRunnerErrorCode, AgentRunnerTypes.AgentRunnerErrorCode>>,
  Assert<Equal<AgentRunnerContracts.AgentRunnerSafeError, AgentRunnerTypes.AgentRunnerSafeError>>,
];

void (0 as unknown as AgentAssertions);
void (0 as unknown as ChatAssertions);
void (0 as unknown as TokenUsageAssertions);
void (0 as unknown as McpAssertions);
void (0 as unknown as SdkAssertions);
void (0 as unknown as OperationAssertions);
void (0 as unknown as CliEnvironmentAssertions);
void (0 as unknown as CliSnapshotAssertions);
void (0 as unknown as OnePieceToolAssertions);
void (0 as unknown as ObservabilityAssertions);
void (0 as unknown as LoopAssertions);
void (0 as unknown as SkillAssertions);
void (0 as unknown as SkillConfigurationAssertions);
void (0 as unknown as SkillOverlayAssertions);
void (0 as unknown as SessionWorkspaceAssertions);
void (0 as unknown as ContextQualityAssertions);
void (0 as unknown as MissionControlAssertions);
void (0 as unknown as AgentRunnerAssertions);

describe("contract conformance", () => {
  it("compiles when committed contracts match frontend service types", () => {
    expect(true).toBe(true);
  });

  it("keeps MCP safety constants aligned with the native runtime contract", () => {
    const expectedErrorCodes = [
      "validation",
      "spawn",
      "timeout",
      "cancelled",
      "protocol",
      "upstream_http",
      "limit_exceeded",
      "transport",
      "cleanup",
    ];
    const expectedLimits = {
      importDocumentBytes: 1024 * 1024,
      importServerEntries: 128,
      configurationCollectionEntries: 128,
      configurationSerializedBytes: 256 * 1024,
      protocolMessageBytes: 2 * 1024 * 1024,
      toolsPerServer: 128,
      catalogSerializedBytes: 2 * 1024 * 1024,
      providerTools: 256,
      toolNameBytes: 256,
      toolDescriptionBytes: 8 * 1024,
      schemaBytes: 128 * 1024,
      jsonDepth: 32,
      toolArgumentsBytes: 256 * 1024,
      toolResultBytes: 1024 * 1024,
      stderrBytes: 64 * 1024,
    };

    expect(contractMcpErrorCodes).toEqual(expectedErrorCodes);
    expect(frontendMcpErrorCodes).toEqual(expectedErrorCodes);
    expect(contractMcpLimits).toEqual(expectedLimits);
    expect(frontendMcpLimits).toEqual(expectedLimits);
  });

  it("keeps effective Skill protocol fixtures aligned", () => {
    expect(contractSkillScopes).toEqual(["global", "workspace"]);
    expect(frontendSkillScopes).toEqual(contractSkillScopes);
    expect(contractSkillLayers).toEqual(["project", "user", "registry", "system"]);
    expect(frontendSkillLayers).toEqual(contractSkillLayers);
    expect(contractSkillReadTools).toEqual(["list_skills", "load_skill", "read_skill_resource"]);
    expect(frontendSkillReadTools).toEqual(contractSkillReadTools);
    expect(contractSkillUriPrefix).toBe("skill://");
    expect(frontendSkillUriPrefix).toBe(contractSkillUriPrefix);
    expect(contractSkillDefaults).toEqual({ type: "role", delivery: "eager" });
    expect(frontendSkillDefaults).toEqual(contractSkillDefaults);
  });
});
