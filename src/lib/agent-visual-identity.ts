import {
  Bot,
  BrainCircuit,
  Code2,
  Compass,
  GitBranch,
  History,
  Languages,
  MousePointer2,
  Orbit,
  Sparkles,
  TerminalSquare,
  Users,
  Waves,
  type LucideIcon,
} from "lucide-react";

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
  "antigravity-cli": { label: "Antigravity CLI", Icon: Orbit, tone: "ucd-agent-antigravity" },
  // The seven expanded CLIs use utility tones like OnePiece: no vendor publishes a mark this
  // repository may embed, so the icon is a neutral glyph and the tone is the only brand cue.
  "qwen-code": { label: "Qwen Code", Icon: Languages, tone: "border-purple-400/60 bg-purple-500/10 text-purple-600" },
  "kimi-cli": { label: "Kimi Code CLI", Icon: Waves, tone: "border-slate-400/60 bg-slate-500/10 text-slate-700" },
  "qoder-cli": { label: "Qoder CLI", Icon: Compass, tone: "border-sky-400/60 bg-sky-500/10 text-sky-600" },
  "codebuddy-code": { label: "CodeBuddy Code", Icon: Users, tone: "border-blue-400/60 bg-blue-500/10 text-blue-600" },
  "copilot-cli": { label: "GitHub Copilot CLI", Icon: GitBranch, tone: "border-zinc-400/60 bg-zinc-500/10 text-zinc-700" },
  "cursor-agent-cli": { label: "Cursor Agent CLI", Icon: MousePointer2, tone: "border-stone-400/60 bg-stone-500/10 text-stone-700" },
  "iflow-cli": { label: "iFlow CLI", Icon: History, tone: "border-amber-400/60 bg-amber-500/10 text-amber-700" },
  onepiece: { label: "OnePiece", Icon: Bot, tone: "border-violet-400/60 bg-violet-500/10 text-violet-600" },
};

export function getAgentVisualIdentity(agentId: string): AgentVisualIdentity {
  return identities[agentId] ?? { label: "Agent", Icon: Bot, tone: "border-border bg-muted text-muted-foreground" };
}
