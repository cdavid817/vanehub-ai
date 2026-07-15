import type { LucideIcon } from "lucide-react";

export interface WorkspaceConversation {
  title: string;
  status: string;
  agents: string;
  date: string;
  active?: boolean;
  archived?: boolean;
}

export interface WorkspaceTool {
  label: string;
  iconName: "shield" | "wrench" | "zap" | "layers" | "sliders" | "users";
  tone: string;
}

export interface WorkspaceAgentNode {
  id: string;
  title: string;
  description: string;
  icon: string;
  x: string;
  tone: string;
}

export interface WorkspaceChatMessage {
  role: string;
  content: string;
  time: string;
}

export interface WorkspaceSnapshot {
  conversations: WorkspaceConversation[];
  tools: WorkspaceTool[];
  agentNodes: WorkspaceAgentNode[];
  chatMessages: WorkspaceChatMessage[];
}

export type WorkspaceToolIconMap = Record<WorkspaceTool["iconName"], LucideIcon>;
