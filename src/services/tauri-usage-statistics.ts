import type { SessionUsageSummary, UsageStatistics } from "../types/chat";

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

function isModernSessionUsageSummary(value: unknown): value is SessionUsageSummary {
  return isRecord(value)
    && typeof value.sessionId === "string"
    && isRecord(value.reported)
    && isRecord(value.estimated)
    && isRecord(value.coverage)
    && typeof value.responseCount === "number"
    && typeof value.generatedAt === "string";
}

export function normalizeTauriUsageStatistics(value: unknown): UsageStatistics {
  if (isModernUsageStatistics(value)) return value;
  throw new Error("The desktop runtime returned an invalid usage-statistics response.");
}

export function normalizeTauriSessionUsageSummary(value: unknown): SessionUsageSummary {
  if (isModernSessionUsageSummary(value)) return value;
  throw new Error("The desktop runtime returned an invalid session-usage response.");
}
