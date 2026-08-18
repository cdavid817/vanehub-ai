import type {
  SessionUsageSummary,
  UsageStatistics,
  UsageStatisticsRange,
} from "../types/chat";
import type {
  TokenUsageDetailsPage,
  TokenUsageDetailsQuery,
  TokenUsageSummary,
  TokenUsageSummaryQuery,
} from "../types/token-usage";

export interface UsageStatisticsService {
  getUsageStatistics(input: { range: UsageStatisticsRange }): Promise<UsageStatistics>;
  getSessionUsageSummary(sessionId: string): Promise<SessionUsageSummary>;
  getTokenUsageSummary(input: TokenUsageSummaryQuery): Promise<TokenUsageSummary>;
  getTokenUsageDetails(input: TokenUsageDetailsQuery): Promise<TokenUsageDetailsPage>;
}
