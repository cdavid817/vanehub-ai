// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { queryWebTokenUsageDetails, queryWebTokenUsageSummary } from "../services/web-token-usage";
import { SessionTokenUsagePane } from "./session-token-usage-pane";

describe("SessionTokenUsagePane", () => {
  beforeEach(async () => {
    await activateAppLanguage("en");
  });

  afterEach(() => vi.restoreAllMocks());

  it("loads bounded safe invocation details only after expansion", async () => {
    vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(queryWebTokenUsageSummary({ sessionId: "web-token-onepiece" }));
    const getDetails = vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(queryWebTokenUsageDetails({ sessionId: "web-token-onepiece", limit: 10 }));
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = userEvent.setup();
    render(<QueryClientProvider client={queryClient}><SessionTokenUsagePane lifecycle="stopped" sessionId="session-1" /></QueryClientProvider>);

    const toggle = await screen.findByRole("button", { name: "Invocation details" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(getDetails).not.toHaveBeenCalled();

    await user.click(toggle);
    await waitFor(() => expect(getDetails).toHaveBeenCalledWith({ sessionId: "session-1", limit: 10 }));
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(await screen.findByText("openai-compatible · reasoning-model")).toBeTruthy();
    expect(screen.getByText("90 Tokens")).toBeTruthy();
    expect(document.body.textContent).not.toContain("credential");
    expect(document.body.textContent).not.toContain("prompt");
  });
});
