import type { UsageStatistics, UsageStatisticsRange } from "../types/chat";

interface LegacyUsageStatistics {
  countedMessages: number;
  countedSessions: number;
  generatedAt: string;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isModernUsageStatistics(value: unknown): value is UsageStatistics {
  return isRecord(value)
    && isRecord(value.reported)
    && isRecord(value.estimated)
    && isRecord(value.coverage)
    && Array.isArray(value.daily)
    && Array.isArray(value.byAgent);
}

function isLegacyUsageStatistics(value: unknown): value is LegacyUsageStatistics {
  return isRecord(value)
    && typeof value.totalTokens === "number"
    && typeof value.inputTokens === "number"
    && typeof value.outputTokens === "number"
    && typeof value.countedMessages === "number"
    && typeof value.countedSessions === "number"
    && typeof value.generatedAt === "string";
}

export function normalizeTauriUsageStatistics(
  value: unknown,
  range: UsageStatisticsRange,
): UsageStatistics {
  if (isModernUsageStatistics(value)) return value;
  if (!isLegacyUsageStatistics(value)) {
    throw new Error("The desktop runtime returned an invalid usage-statistics response.");
  }

  return {
    range,
    reported: {
      inputTokens: value.inputTokens,
      outputTokens: value.outputTokens,
      cacheReadTokens: 0,
      cacheCreationTokens: 0,
      totalTokens: value.totalTokens,
    },
    estimated: {
      inputCharacters: 0,
      outputCharacters: 0,
      totalCharacters: 0,
    },
    coverage: {
      reportedResponses: value.countedMessages,
      estimatedResponses: 0,
      totalResponses: value.countedMessages,
      reportedPercent: value.countedMessages === 0 ? 0 : 100,
    },
    countedSessions: value.countedSessions,
    daily: [],
    byAgent: [],
    generatedAt: value.generatedAt,
  };
}
