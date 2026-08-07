import type { AgentWithModelFamily } from "./agent-model-family";
import { isSameFamily } from "./model-family";

export interface ReviewerRecommendation {
  agents: AgentWithModelFamily[];
  /**
   * True when no cross-family Agent was available and same-family ones are offered instead. The
   * caller must say so rather than presenting the fallback as a satisfied preference.
   */
  degraded: boolean;
}

/**
 * Recommends who can review the work of `agentUnderReviewId`, preferring a different model family
 * because same-family models make correlated errors and tend to agree with each other.
 *
 * Degrades openly instead of returning nothing: a strict version leaves a user unable to assign any
 * reviewer at all, which is worse than a same-family reviewer plus a clear notice.
 */
export function recommendReviewerAgents(
  agents: AgentWithModelFamily[],
  agentUnderReviewId: string,
): ReviewerRecommendation {
  const underReview = agents.find((agent) => agent.id === agentUnderReviewId);
  const candidates = agents.filter(
    (agent) => agent.id !== agentUnderReviewId && agent.availabilityState === "available",
  );
  if (!underReview) return { agents: candidates, degraded: false };

  const crossFamily = candidates.filter(
    (agent) => !isSameFamily(agent.modelFamily, underReview.modelFamily),
  );
  if (crossFamily.length > 0) return { agents: crossFamily, degraded: false };
  return { agents: candidates, degraded: candidates.length > 0 };
}
