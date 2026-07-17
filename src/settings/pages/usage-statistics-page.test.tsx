import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { UsageStatisticsPage } from "./usage-statistics-page";

describe("UsageStatisticsPage", () => {
  it("renders localized title, range controls, and first-version constraints", () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <UsageStatisticsPage />
      </QueryClientProvider>,
    );

    expect(html).toContain("使用统计");
    expect(html).toContain("近 30 天");
    expect(html).toContain("总 Token");
    expect(html).toContain("第一版约束");
  });
});
