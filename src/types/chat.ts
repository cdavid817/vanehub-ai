import type { TurnStatusEvent } from "../services/turn-status";
import type { InteractionMode } from "./agent";
import type { PolicyTemplateName } from "./permissions";

export type MessageRole = "user" | "assistant" | "system" | "tool";

export type MessageStatus = "pending" | "streaming" | "completed" | "failed" | "cancelled";
export type MessageFeedbackState = "helpful" | "unhelpful" | "corrected";

export interface MessageFeedback {
  state: MessageFeedbackState | null;
  revision: number;
  correctionNote?: string;
}

export interface SaveMessageFeedbackInput {
  messageId: string;
  expectedRevision: number;
  state: MessageFeedbackState | null;
  correctionNote?: string;
}

export type ReasoningDepth = "low" | "medium" | "high" | "max";

export type SessionExecutionMode = "inherit" | "plan" | "execute";
export type EffectiveExecutionPolicy = "readonly" | "ask" | "allow";

export interface ModelInfo {
  id: string;
  label: string;
  providerId: string;
  description?: string;
  supportsReasoning: boolean;
  maxReasoningDepth: ReasoningDepth;
  supportsLongContext: boolean;
}

export interface ChatConfig {
  agentId: string;
  interactionMode: InteractionMode;
  executionMode: SessionExecutionMode;
  agentPolicy?: PolicyTemplateName;
  effectiveExecutionPolicy?: EffectiveExecutionPolicy;
  providerId?: string;
  modelId?: string;
  reasoningDepth?: ReasoningDepth;
  streaming: boolean;
  thinking: boolean;
  longContext: boolean;
}

export interface ToolUseBlock {
  id: string;
  name: string;
  input?: unknown;
  output?: unknown;
  /**
   * `awaiting_input` is a question waiting on the user, distinct from `awaiting_approval`: the
   * affordance differs (choose one of N versus allow/deny), and rendering one as the other would
   * show a security prompt for a clarification.
   */
  status: "pending" | "running" | "awaiting_approval" | "awaiting_input" | "completed" | "failed" | "cancelled";
  skillProvenance?: SkillToolUseProvenance;
}

export interface SkillToolUseProvenance {
  skillId: string;
  toolId: string;
  revision: string;
  sourceScope: "global" | "workspace";
  workspacePath?: string;
  redactedResultSummary?: "completed" | "failed" | "cancelled";
}

export type RichBlockKind =
  | "card"
  | "diff"
  | "checklist"
  | "media_gallery"
  | "audio"
  | "interactive"
  | "html_widget"
  | "file";

interface RichBlockBase {
  id: string;
  kind: RichBlockKind;
  v: 1;
}

export interface RichCardBlock extends RichBlockBase {
  kind: "card";
  title: string;
  bodyMarkdown?: string;
  tone?: "info" | "success" | "warning" | "danger";
  fields?: Array<{ label: string; value: string }>;
  meta?: Record<string, unknown>;
}

export interface RichDiffBlock extends RichBlockBase {
  kind: "diff";
  filePath: string;
  diff: string;
  languageHint?: string;
}

export interface RichChecklistBlock extends RichBlockBase {
  kind: "checklist";
  title?: string;
  items: Array<{ id: string; text: string; checked?: boolean }>;
}

export interface RichMediaGalleryBlock extends RichBlockBase {
  kind: "media_gallery";
  title?: string;
  items: Array<{ url: string; alt?: string; caption?: string }>;
}

export interface RichAudioBlock extends RichBlockBase {
  kind: "audio";
  url: string;
  text?: string;
  title?: string;
  durationSec?: number;
  mimeType?: string;
}

export interface RichInteractiveOption {
  id: string;
  label: string;
  description?: string;
}

export interface RichInteractiveBlock extends RichBlockBase {
  kind: "interactive";
  interactiveType: "select" | "multi-select" | "card-grid" | "confirm";
  title?: string;
  description?: string;
  options: RichInteractiveOption[];
  maxSelect?: number;
  allowRandom?: boolean;
  messageTemplate?: string;
  disabled?: boolean;
  selectedIds?: string[];
  groupId?: string;
}

export interface RichHtmlWidgetBlock extends RichBlockBase {
  kind: "html_widget";
  html: string;
  title?: string;
  height?: number;
}

export interface RichFileBlock extends RichBlockBase {
  kind: "file";
  url: string;
  fileName: string;
  mimeType?: string;
  fileSize?: number;
}

export type RichBlock =
  | RichCardBlock
  | RichDiffBlock
  | RichChecklistBlock
  | RichMediaGalleryBlock
  | RichAudioBlock
  | RichInteractiveBlock
  | RichHtmlWidgetBlock
  | RichFileBlock;

export interface TokenUsage {
  input: number;
  output: number;
}

/**
 * Mirrors `MAX_FILE_REFERENCES` in the sessions domain. The domain remains the authority
 * and still rejects an over-limit message; this exists so the composer can refuse the
 * attachment up front instead of surfacing that rejection at send time.
 */
export const MAX_CHAT_FILE_REFERENCES = 5;

export interface ChatFileReference {
  id: string;
  path: string;
  name: string;
  sizeBytes?: number | null;
  contentHash?: string | null;
  // Absent means the whole file. Both bounds are 1-based and inclusive, and either both
  // are present or neither is.
  startLine?: number;
  endLine?: number;
}

export type UsageStatisticsRange = "today" | "last7Days" | "last30Days" | "all";

export interface ReportedTokenTotals {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  totalTokens: number;
}

export interface EstimatedCharacterTotals {
  inputCharacters: number;
  outputCharacters: number;
  totalCharacters: number;
}

export interface UsageCoverage {
  reportedResponses: number;
  estimatedResponses: number;
  totalResponses: number;
  reportedPercent: number;
}

export interface UsageStatisticsPoint {
  date: string;
  reported: ReportedTokenTotals;
  estimated: EstimatedCharacterTotals;
  responseCount: number;
}

export interface UsageAgentBreakdown {
  agentId: string;
  reported: ReportedTokenTotals;
  estimated: EstimatedCharacterTotals;
  responseCount: number;
}

export interface UsageStatistics {
  range: UsageStatisticsRange;
  reported: ReportedTokenTotals;
  estimated: EstimatedCharacterTotals;
  coverage: UsageCoverage;
  countedSessions: number;
  daily: UsageStatisticsPoint[];
  byAgent: UsageAgentBreakdown[];
  generatedAt: string;
}

export interface SessionUsageSummary {
  sessionId: string;
  reported: ReportedTokenTotals;
  estimated: EstimatedCharacterTotals;
  coverage: UsageCoverage;
  responseCount: number;
  generatedAt: string;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: MessageRole;
  /** Stable participant identity for messages created after participant migration. */
  speakerSeatId?: string;
  /**
   * Legacy attribution for messages that predate stable participant ids.
   */
  seatIndex?: number;
  content: string;
  status: MessageStatus;
  toolUse?: ToolUseBlock[];
  thinkingContent?: string;
  richBlocks?: RichBlock[];
  tokenUsage?: TokenUsage;
  fileReferences?: ChatFileReference[];
  error?: string;
  createdAt: string;
  updatedAt: string;
  sessionSequence: number;
  executionRunId: string | null;
  feedback?: MessageFeedback;
}

export type ChatStreamEvent =
  | { type: "started"; sessionId: string; messageId: string }
  | { type: "token"; sessionId: string; messageId: string; contentDelta: string }
  | { type: "thinking"; sessionId: string; messageId: string; contentDelta: string }
  | { type: "tool_use"; sessionId: string; messageId: string; toolUse: ToolUseBlock }
  | { type: "rich_block"; sessionId: string; messageId: string; block: RichBlock }
  | { type: "completed"; sessionId: string; messageId: string; tokenUsage?: TokenUsage }
  | { type: "failed"; sessionId: string; messageId: string; error: string }
  | { type: "cancelled"; sessionId: string; messageId: string }
  | { type: "turn_status"; sessionId: string; status: TurnStatusEvent };

export interface SendMessageInput {
  sessionId: string;
  content: string;
  config: ChatConfig;
  fileReferences?: ChatFileReference[];
  runner?: import("./agent-runner").AgentRunnerSelection;
}
