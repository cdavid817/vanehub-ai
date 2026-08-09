import type { UsageStatistics } from "../../../types/chat";

export const usagePollingInterval = 30_000;

/**
 * The settings shell keeps every visited page mounted and hides the inactive ones, so a page that
 * polls unconditionally keeps querying for a surface nobody can see. Gate the interval on the page
 * actually being the active tab.
 */
export function usageRefetchInterval(isActive: boolean): number | false {
  return isActive ? usagePollingInterval : false;
}

export function preserveUsageData(previous: UsageStatistics | undefined) {
  return previous;
}
