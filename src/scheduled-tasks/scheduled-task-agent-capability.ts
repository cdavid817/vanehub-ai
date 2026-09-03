import type { AgentRegistryEntry } from "../types/agent";

/**
 * 19.6's own "capability notice": an honest availability check of the task's own `agentId`
 * against the real Agent registry, the same shape Evaluation Center already established for the
 * identical question (`isEvaluationAgentIncompatible`, `evaluation-agent-filters.ts`) --
 * `availabilityState !== "available"` is not a scheduling-specific concept, so this reuses that
 * exact test rather than inventing a second one. Unlike Evaluation's own selector (which only
 * ever sees agents that are still *in* the registry, since the reader picks from a live list),
 * a scheduled task's `agentId` is a durable reference that can outlive the agent it names --
 * uninstalling a CLI Agent, for example, removes it from `agents` entirely. `undefined` is that
 * stronger case: not merely unavailable, but gone, which callers distinguish from a defined-but-
 * unavailable entry via `ScheduledTaskAgentCapability`'s own `"missing"` reason.
 */
export type ScheduledTaskAgentCapabilityReason = "missing" | "unavailable";

export interface ScheduledTaskAgentCapability {
  reason: ScheduledTaskAgentCapabilityReason;
}

export function scheduledTaskAgentCapability(agent: AgentRegistryEntry | undefined): ScheduledTaskAgentCapability | null {
  if (!agent) return { reason: "missing" };
  if (agent.availabilityState !== "available") return { reason: "unavailable" };
  return null;
}
