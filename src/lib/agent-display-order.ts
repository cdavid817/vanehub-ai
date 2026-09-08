/**
 * The original five keep their positions; the seven added by `extend-cli-providers-with-acp`
 * follow them (domestic CLIs first, then Copilot and Cursor, then the legacy iFlow entry), and
 * OnePiece stays last. Existing defaults are unchanged because the first entry is unchanged.
 */
export const settingsAgentPriority = [
  "claude-code",
  "codex-cli",
  "opencode",
  "antigravity-cli",
  "gemini-cli",
  "qwen-code",
  "kimi-cli",
  "qoder-cli",
  "codebuddy-code",
  "copilot-cli",
  "cursor-agent-cli",
  "iflow-cli",
  "onepiece",
] as const;

export const createSessionCliPriority = settingsAgentPriority.slice(0, 12);

export function orderByAgentPriority<T>(
  items: readonly T[],
  getAgentId: (item: T) => string,
  priority: readonly string[] = settingsAgentPriority,
): T[] {
  const ranks = new Map(priority.map((agentId, index) => [agentId, index]));
  return items
    .map((item, index) => ({ index, item, rank: ranks.get(getAgentId(item)) ?? Number.MAX_SAFE_INTEGER }))
    .sort((left, right) => left.rank - right.rank || left.index - right.index)
    .map(({ item }) => item);
}
