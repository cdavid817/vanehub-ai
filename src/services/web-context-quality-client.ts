import type { AgentService } from "./agent-service";
import type { ContextQualityService } from "./scheduled-task-service";
import { getWebContextQualitySummary, listWebContextQualityHistory } from "./web-context-quality";

export const webContextQualityClient: ContextQualityService = {
  async listContextQualityHistory(input) {
    return listWebContextQualityHistory(input);
  },

  async getContextQualitySummary(input) {
    return getWebContextQualitySummary(input);
  },

  async listContextEvidenceManifests(input) {
    const manifest = {
      sessionId: input.sessionId ?? "web-context-session",
      turnId: "web-context-turn",
      generationId: "web-context-generation",
      policyVersion: "context-engine-v1",
      evidenceBudget: 4096,
      occupiedTokens: 768,
      selected: [{ id: "web-definition", sourceKind: "retrieval", sourceRef: "src/example.ts", startLine: 12, endLine: 28, symbol: "example", tokenEstimate: 512, reasonCodes: ["semantic-match", "symbol-relation"] }],
      rejected: [{ id: "web-memory", reasonCode: "budget-rejected" }],
      sourceOutcomes: { retrieval: "ready" as const, lsp: "unavailable" as const },
      duplicateTokensSaved: 256,
      collectionLatencyBucket: "under-50ms",
      rankingLatencyBucket: "under-10ms",
      compactionTriggered: false,
      runtime: "web-mock" as const,
    };
    return { items: input.cursor ? [] : [manifest], nextCursor: null };
  },

  async getContextEvidenceManifest(this: AgentService, generationId) {
    const page = await this.listContextEvidenceManifests({ cursor: null, limit: 1 });
    return page.items.find((item) => item.generationId === generationId) ?? null;
  },
};
