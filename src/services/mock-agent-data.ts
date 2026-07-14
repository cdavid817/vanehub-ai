import type { AgentRegistryEntry, WorkflowState } from "../types/agent";

export const mockAgents: AgentRegistryEntry[] = [
  {
    id: "claude-code",
    displayName: "Claude Code",
    provider: "Anthropic",
    managedSdkDependencyId: "claude-sdk",
    launch: { kind: "cli", command: "claude", executableName: "claude" },
    supportedInteractionModes: ["cli", "native-desktop"],
    availabilityState: "unknown",
    capabilityTags: ["coding", "cli", "agent"],
  },
  {
    id: "opencode",
    displayName: "OpenCode",
    provider: "OpenCode",
    launch: { kind: "cli", command: "opencode", executableName: "opencode" },
    supportedInteractionModes: ["cli"],
    availabilityState: "unknown",
    capabilityTags: ["coding", "cli", "open-source"],
  },
  {
    id: "codex-cli",
    displayName: "Codex CLI",
    provider: "OpenAI",
    managedSdkDependencyId: "codex-sdk",
    launch: { kind: "cli", command: "codex", executableName: "codex" },
    supportedInteractionModes: ["cli", "native-desktop"],
    availabilityState: "unknown",
    capabilityTags: ["coding", "cli", "agent"],
  },
  {
    id: "gemini-cli",
    displayName: "Gemini CLI",
    provider: "Google",
    launch: { kind: "cli", command: "gemini", executableName: "gemini" },
    supportedInteractionModes: ["cli", "browser"],
    availabilityState: "unknown",
    capabilityTags: ["coding", "cli", "browser"],
  },
];

export const mockWorkflowState: WorkflowState = {
  activeAgentId: null,
  activeInteractionMode: null,
  lifecycleState: "idle",
  intent: "Current development workflow",
};
