import type { UsageStatistics, UsageStatisticsRange } from "../types/chat";
import type { UsageStatisticsService } from "./usage-statistics-service";
import { aggregateSessionUsageRecords, aggregateUsageRecords, type UsageRecord } from "./usage-statistics";
import { queryWebTokenUsageDetails, queryWebTokenUsageSummary } from "./web-token-usage";
import { daysAgoIso } from "./web-mock-clock";
import { findWebSession } from "./web-session-state";

const representativeUsageRecords: UsageRecord[] = [
  {
    messageId: "web-usage-reported",
    sessionId: "web-usage-session-codex",
    agentId: "codex-cli",
    accountingKind: "reported",
    inputCount: 100,
    outputCount: 40,
    cacheReadCount: 10,
    cacheCreationCount: 5,
    occurredAt: daysAgoIso(1),
  },
  {
    messageId: "web-usage-estimated",
    sessionId: "web-usage-session-claude",
    agentId: "claude-code",
    accountingKind: "estimated",
    inputCount: 1_000,
    outputCount: 400,
    cacheReadCount: 0,
    cacheCreationCount: 0,
    occurredAt: daysAgoIso(2),
  },
];

function aggregateWebUsageStatistics(range: UsageStatisticsRange): UsageStatistics {
  return aggregateUsageRecords(representativeUsageRecords, range);
}

export const webUsageStatisticsClient: UsageStatisticsService = {
  async getUsageStatistics(input) {
    return aggregateWebUsageStatistics(input.range);
  },

  async getSessionUsageSummary(sessionId: string) {
    findWebSession(sessionId);
    const generated = aggregateSessionUsageRecords(representativeUsageRecords, sessionId);
    return generated;
  },

  async getTokenUsageSummary(input) {
    if (!input.sessionId) return queryWebTokenUsageSummary(input);
    const session = findWebSession(input.sessionId);
    return queryWebTokenUsageSummary({
      ...input,
      sessionId: session.agentId === "onepiece" ? "web-token-onepiece" : "web-token-cli",
    });
  },

  async getTokenUsageDetails(input) {
    if (!input.sessionId) return queryWebTokenUsageDetails(input);
    const session = findWebSession(input.sessionId);
    return queryWebTokenUsageDetails({
      ...input,
      sessionId: session.agentId === "onepiece" ? "web-token-onepiece" : "web-token-cli",
    });
  },
};
