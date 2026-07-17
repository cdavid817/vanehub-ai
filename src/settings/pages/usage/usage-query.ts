import type { UsageStatistics } from "../../../types/chat";

export const usagePollingInterval = 30_000;

export function preserveUsageData(previous: UsageStatistics | undefined) {
  return previous;
}
