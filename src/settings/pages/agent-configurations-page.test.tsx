// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { getCliConfigPresets } from "../../config/cli-agent-provider-presets";
import { createAgentServiceDouble, renderWithAppProviders } from "../../test/render";
import type { CliConfigAgentId } from "../../types/cli-agent-config";
import { AgentConfigurationsPage } from "./agent-configurations-page";

describe("AgentConfigurationsPage", () => {
  it("honors the originating Agent and isolates data when tabs change", async () => {
    const listProfiles = vi.fn(async () => []);
    const listPresets = vi.fn(async (agentId: CliConfigAgentId) => getCliConfigPresets(agentId));
    const getStatus = vi.fn(async (agentId: CliConfigAgentId) => ({ agentId, appliedProfileId: null, driftState: "detached" as const, resolvedPaths: [], lastAppliedAt: null, simulated: true, startupSync: { agentId, state: "unavailable" as const, imported: 0, updated: 0, skipped: 0, warnings: [], synchronizedAt: null, simulated: true } }));
    const selectAgent = vi.fn();
    const service = createAgentServiceDouble({
      listCliConfigProfiles: listProfiles,
      listCliConfigPresets: listPresets,
      getCliConfigStatus: getStatus,
      selectAgent,
    });
    const { user } = renderWithAppProviders(<AgentConfigurationsPage navigationTarget={{ cliConfigAgentId: "codex-cli" }} onNavigate={vi.fn()} searchTerm="" service={service} />);

    await waitFor(() => expect(getStatus).toHaveBeenCalledWith("codex-cli"));
    expect(screen.getByRole("tab", { name: "Codex CLI" }).getAttribute("aria-selected")).toBe("true");
    await user.click(screen.getByRole("tab", { name: "OpenCode" }));
    await waitFor(() => expect(getStatus).toHaveBeenCalledWith("opencode"));
    expect(screen.getByRole("tab", { name: "OpenCode" }).getAttribute("aria-selected")).toBe("true");
    expect(selectAgent).not.toHaveBeenCalled();
  });
});
