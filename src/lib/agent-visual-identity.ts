import { Bot, BrainCircuit, Code2, Sparkles, TerminalSquare, type LucideIcon } from "lucide-react";

export interface AgentVisualIdentity {
  label: string;
  Icon: LucideIcon;
  tone: string;
}

const identities: Record<string, AgentVisualIdentity> = {
  "claude-code": { label: "Claude Code", Icon: Sparkles, tone: "ucd-agent-claude" },
  "codex-cli": { label: "Codex CLI", Icon: Code2, tone: "ucd-agent-codex" },
  "gemini-cli": { label: "Gemini CLI", Icon: BrainCircuit, tone: "ucd-agent-gemini" },
  opencode: { label: "OpenCode", Icon: TerminalSquare, tone: "ucd-agent-opencode" },
};

export function getAgentVisualIdentity(agentId: string): AgentVisualIdentity {
  return identities[agentId] ?? { label: "Agent", Icon: Bot, tone: "border-border bg-muted text-muted-foreground" };
}
