import type {
  EstimatedCharacterTotals,
  ReportedTokenTotals,
  UsageAgentBreakdown,
  UsageStatistics,
  UsageStatisticsPoint,
  UsageStatisticsRange,
} from "../types/chat";

export type UsageAccountingKind = "reported" | "estimated";

export interface UsageRecord {
  messageId: string;
  sessionId: string;
  agentId: string;
  accountingKind: UsageAccountingKind;
  inputCount: number;
  outputCount: number;
  cacheReadCount: number;
  cacheCreationCount: number;
  occurredAt: string;
}

function emptyReported(): ReportedTokenTotals {
  return {
    inputTokens: 0,
    outputTokens: 0,
    cacheReadTokens: 0,
    cacheCreationTokens: 0,
    totalTokens: 0,
  };
}

function emptyEstimated(): EstimatedCharacterTotals {
  return {
    inputCharacters: 0,
    outputCharacters: 0,
    totalCharacters: 0,
  };
}

function addRecord(
  reported: ReportedTokenTotals,
  estimated: EstimatedCharacterTotals,
  record: UsageRecord,
) {
  if (record.accountingKind === "reported") {
    reported.inputTokens += record.inputCount;
    reported.outputTokens += record.outputCount;
    reported.cacheReadTokens += record.cacheReadCount;
    reported.cacheCreationTokens += record.cacheCreationCount;
    reported.totalTokens =
      reported.inputTokens +
      reported.outputTokens +
      reported.cacheReadTokens +
      reported.cacheCreationTokens;
    return;
  }
  estimated.inputCharacters += record.inputCount;
  estimated.outputCharacters += record.outputCount;
  estimated.totalCharacters = estimated.inputCharacters + estimated.outputCharacters;
}

export function usageRangeStart(range: UsageStatisticsRange, now = new Date()) {
  if (range === "all") return null;
  const start = new Date(now);
  start.setHours(0, 0, 0, 0);
  if (range === "last7Days") {
    start.setDate(start.getDate() - 6);
  } else if (range === "last30Days") {
    start.setDate(start.getDate() - 29);
  }
  return start;
}

export function localDateKey(value: Date) {
  const year = value.getFullYear();
  const month = String(value.getMonth() + 1).padStart(2, "0");
  const day = String(value.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function aggregateUsageRecords(
  records: UsageRecord[],
  range: UsageStatisticsRange,
  now = new Date(),
): UsageStatistics {
  const start = usageRangeStart(range, now);
  const selected = records.filter((record) => {
    const occurredAt = new Date(record.occurredAt);
    return Number.isFinite(occurredAt.getTime()) && (!start || occurredAt >= start);
  });
  const reported = emptyReported();
  const estimated = emptyEstimated();
  const sessionIds = new Set<string>();
  const dailyMap = new Map<string, UsageStatisticsPoint>();
  const agentMap = new Map<string, UsageAgentBreakdown>();
  let reportedResponses = 0;
  let estimatedResponses = 0;

  for (const record of selected) {
    addRecord(reported, estimated, record);
    sessionIds.add(record.sessionId);
    if (record.accountingKind === "reported") {
      reportedResponses += 1;
    } else {
      estimatedResponses += 1;
    }

    const date = localDateKey(new Date(record.occurredAt));
    const point = dailyMap.get(date) ?? {
      date,
      reported: emptyReported(),
      estimated: emptyEstimated(),
      responseCount: 0,
    };
    addRecord(point.reported, point.estimated, record);
    point.responseCount += 1;
    dailyMap.set(date, point);

    const agent = agentMap.get(record.agentId) ?? {
      agentId: record.agentId,
      reported: emptyReported(),
      estimated: emptyEstimated(),
      responseCount: 0,
    };
    addRecord(agent.reported, agent.estimated, record);
    agent.responseCount += 1;
    agentMap.set(record.agentId, agent);
  }

  const totalResponses = reportedResponses + estimatedResponses;
  return {
    range,
    reported,
    estimated,
    coverage: {
      reportedResponses,
      estimatedResponses,
      totalResponses,
      reportedPercent: totalResponses === 0 ? 0 : (reportedResponses / totalResponses) * 100,
    },
    countedSessions: sessionIds.size,
    daily: [...dailyMap.values()].sort((left, right) => left.date.localeCompare(right.date)),
    byAgent: [...agentMap.values()].sort(
      (left, right) => right.responseCount - left.responseCount || left.agentId.localeCompare(right.agentId),
    ),
    generatedAt: now.toISOString(),
  };
}
