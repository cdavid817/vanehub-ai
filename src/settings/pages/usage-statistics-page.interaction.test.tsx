// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { agentService } from "../../services/runtime-agent-client";
import { queryWebTokenUsageSummary } from "../../services/web-token-usage";
import { UsageStatisticsPage } from "./usage-statistics-page";
import { UsageEmptyState, UsageLoadError } from "./usage/usage-status";

function renderPage(initial?: ReturnType<typeof queryWebTokenUsageSummary>) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  if (initial) {
    queryClient.setQueryData(["token-usage-options", "last30Days"], initial);
    queryClient.setQueryData(["token-usage-summary", "last30Days", {}], initial);
  }
  return render(<QueryClientProvider client={queryClient}><UsageStatisticsPage /></QueryClientProvider>);
}

describe("UsageStatisticsPage interactions", () => {
  beforeEach(async () => {
    await activateAppLanguage("en");
  });

  afterEach(() => vi.restoreAllMocks());

  it("applies Agent, quality, and status as one filter set", async () => {
    const getSummary = vi.spyOn(agentService, "getTokenUsageSummary").mockImplementation(async (input) => (
      queryWebTokenUsageSummary({ ...input, rangeStart: undefined, rangeEnd: undefined })
    ));
    const user = userEvent.setup();
    renderPage(queryWebTokenUsageSummary({}));

    await user.selectOptions(await screen.findByRole("combobox", { name: "Agent" }), "onepiece");
    await user.selectOptions(screen.getByRole("combobox", { name: "Quality" }), "reported");
    await user.selectOptions(screen.getByRole("combobox", { name: "Status" }), "failed");

    await waitFor(() => expect(getSummary).toHaveBeenCalledWith(expect.objectContaining({
      agentId: "onepiece",
      quality: "reported",
      status: "failed",
    })));
    expect(await screen.findByText("Tool continuation")).toBeTruthy();
    expect(screen.getAllByText("90").length).toBeGreaterThan(0);
  });

  it("renders localized semantic empty and error states instead of blank panels", () => {
    render(<><UsageEmptyState /><UsageLoadError error={new Error("offline")} /></>);
    expect(screen.getByRole("status").textContent).toContain("No usage records in the selected time range");
    expect(screen.getByRole("alert").textContent).toContain("Failed to load usage statistics: offline");
  });

  it("reports an error status for its nav entry once the usage query fails, and null beforehand (task 12.16)", async () => {
    vi.spyOn(agentService, "getTokenUsageSummary").mockRejectedValue(new Error("offline"));
    const onStatusChange = vi.fn();
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={queryClient}>
        <UsageStatisticsPage onStatusChange={onStatusChange} />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith(null));
    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
      kind: "error",
      labelKey: "usage.status.error",
    }));
  });
});
