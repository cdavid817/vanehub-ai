export const settingsAgentPriority = [
  "claude-code",
  "codex-cli",
  "opencode",
  "antigravity-cli",
  "gemini-cli",
  "onepiece",
] as const;

export const createSessionCliPriority = settingsAgentPriority.slice(0, 5);

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
