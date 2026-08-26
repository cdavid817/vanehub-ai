import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import { AgentMemorySection } from "./agent-memory-section";
import { MemoryListSection } from "./memory-list-section";

/**
 * The Memory destination.
 *
 * Two panels because they answer to different owners: the toggles are policy the runtime reads
 * before a generation, and the list is the store's contents. Task 11.3 adds the detail panel and
 * 11.6 the scoped reset, both alongside the list rather than inside the toggles.
 */
export function PersonalizationMemoryView({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  return (
    <div className="grid gap-5">
      <AgentMemorySection />
      <MemoryListSection service={service} />
    </div>
  );
}
