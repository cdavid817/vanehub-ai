import { agentRunIsTerminal, type AgentRun } from "../../types/agent-run";

export function agentRunElapsed(run: AgentRun, now = Date.now()): string {
  const startedAt = Date.parse(run.createdAt);
  if (Number.isNaN(startedAt)) return "0:00";
  const terminalAt = Date.parse(run.updatedAt);
  const endedAt = agentRunIsTerminal(run) && !Number.isNaN(terminalAt) ? terminalAt : now;
  const elapsedMs = Math.max(0, endedAt - startedAt);
  return `${Math.floor(elapsedMs / 60_000)}:${String(Math.floor(elapsedMs / 1_000) % 60).padStart(2, "0")}`;
}
