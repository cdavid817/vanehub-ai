import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import { AgentMemorySection } from "./agent-memory-section";
import { CandidateReviewSection } from "./candidate-review-section";
import { MemoryListSection } from "./memory-list-section";

/**
 * The Memory destination.
 *
 * Three panels, in the order a decision travels: the policy the runtime reads before a
 * generation, the proposals waiting on a person, and the store's contents. The review queue sits
 * above the list because approving something is what puts it there.
 */
export function PersonalizationMemoryView({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  return (
    <div className="grid gap-5">
      <AgentMemorySection />
      <CandidateReviewSection service={service} />
      <MemoryListSection service={service} />
    </div>
  );
}
