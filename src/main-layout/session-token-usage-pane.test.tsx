// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { queryWebTokenUsageDetails, queryWebTokenUsageSummary } from "../services/web-token-usage";
import type { TokenUsageDetailsPage } from "../types/token-usage";
import { SessionTokenUsagePane } from "./session-token-usage-pane";

/**
 * Hand-built rather than reusing `queryWebTokenUsageDetails`'s own fixture set: every existing
 * fixture's `providerTotal` is small (40-200) and would not exercise locale grouping, and the
 * fixture module's totals are shared with other suites (`usage-facet.test.tsx`,
 * `overview-facet.test.tsx`) that assert on its aggregate sums -- adding a large-number fixture
 * entry there would risk silently shifting those.
 */
function detailsPageWithLargeProviderTotal(): TokenUsageDetailsPage {
  return {
    schemaVersion: 1,
    invocations: [{
      id: "inv-large", generationId: null, runId: null, operationId: null,
      sessionId: "session-1", messageId: null, agentId: "onepiece",
      providerId: "anthropic", profileId: null, endpointId: null, modelId: "claude-sonnet-4-5",
      interactionKind: "native-api", purpose: "assistant-initial", requestSequence: 1, attempt: 1,
      status: "succeeded", startedAt: "2026-01-01T00:00:00.000Z", completedAt: "2026-01-01T00:00:01.000Z",
    }],
    observations: [{
      id: "obs-large", invocationId: "inv-large", quality: "reported", unit: "tokens",
      measurementKind: "cumulative-snapshot",
      dimensions: { input: 1_000_000, output: 234_567, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 1_234_567 },
      cacheOverlap: "exclusive", reasoningOverlap: "exclusive",
      normalizationVersion: "test-v1", source: "test", sourceRevision: null,
      eventAt: "2026-01-01T00:00:01.000Z", observedAt: "2026-01-01T00:00:01.000Z",
    }],
    nextCursor: null,
  };
}

describe("SessionTokenUsagePane", () => {
  beforeEach(async () => {
    await activateAppLanguage("en");
  });

  afterEach(() => vi.restoreAllMocks());

  it("formats a large invocation token total with locale-aware grouping, not a raw number", async () => {
    vi.spyOn(agentService, "getTokenUsageSummary").mockResolvedValue(queryWebTokenUsageSummary({ sessionId: "web-token-onepiece" }));
    vi.spyOn(agentService, "getTokenUsageDetails").mockResolvedValue(detailsPageWithLargeProviderTotal());
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = userEvent.setup();
    render(<QueryClientProvider client={queryClient}><SessionTokenUsagePane lifecycle="stopped" sessionId="session-1" /></QueryClientProvider>);

    await user.click(await screen.findByRole("button", { name: "Invocation details" }));
    expect(await screen.findByText("1,234,567 Tokens")).toBeTruthy();
    expect(screen.queryByText("1234567 Tokens")).toBeNull();
  });

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
