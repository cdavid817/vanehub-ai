import type { InteractionMode } from "./agent";

export type MessageRole = "user" | "assistant" | "system" | "tool";

export type MessageStatus = "pending" | "streaming" | "completed" | "failed" | "cancelled";

export type ReasoningDepth = "low" | "medium" | "high" | "max";

export type PermissionMode = "default" | "plan" | "agent" | "auto";

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
  permissionMode: PermissionMode;
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
  status: "pending" | "running" | "completed" | "failed";
}

export interface TokenUsage {
  input: number;
  output: number;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: MessageRole;
  content: string;
  status: MessageStatus;
  toolUse?: ToolUseBlock[];
  thinkingContent?: string;
  tokenUsage?: TokenUsage;
  error?: string;
  createdAt: string;
  updatedAt: string;
}

export type ChatStreamEvent =
  | { type: "started"; sessionId: string; messageId: string }
  | { type: "token"; sessionId: string; messageId: string; contentDelta: string }
  | { type: "thinking"; sessionId: string; messageId: string; contentDelta: string }
  | { type: "tool_use"; sessionId: string; messageId: string; toolUse: ToolUseBlock }
  | { type: "completed"; sessionId: string; messageId: string; tokenUsage?: TokenUsage }
  | { type: "failed"; sessionId: string; messageId: string; error: string }
  | { type: "cancelled"; sessionId: string; messageId: string };

export interface SendMessageInput {
  sessionId: string;
  content: string;
  config: ChatConfig;
}
