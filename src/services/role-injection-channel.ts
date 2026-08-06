/**
 * How a seat's role instruction reaches its Agent.
 *
 * `native` uses the CLI's own system-prompt mechanism, which survives context compaction. That
 * matters more than it looks: injected per turn instead, a long session would compact the role away
 * and the Agent would quietly stop being the reviewer without anyone noticing.
 *
 * When no native channel exists the seat degrades to per-turn injection and says so, rather than
 * pretending the role is durable.
 */
export interface RoleInjectionChannel {
  kind: "native" | "per-turn";
  compactionImmune: boolean;
}

/** Claude takes `--system-prompt-file`; Codex takes `-c developer_instructions`. */
const nativeChannelAgents = new Set(["claude-code", "codex-cli"]);

export function roleInjectionChannel(agentId: string): RoleInjectionChannel {
  return nativeChannelAgents.has(agentId)
    ? { kind: "native", compactionImmune: true }
    : { kind: "per-turn", compactionImmune: false };
}
