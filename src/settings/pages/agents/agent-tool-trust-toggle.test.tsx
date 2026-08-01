// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { AgentRegistryEntry } from "../../../types/agent";
import { AgentToolTrustToggle } from "./agent-tool-trust-toggle";

const agent: AgentRegistryEntry = {
  id: "my-api-agent",
  displayName: "My API Agent",
  provider: "Anthropic",
  launch: { kind: "api" },
  supportedInteractionModes: ["api"],
  availabilityState: "available",
  capabilityTags: ["api"],
};

describe("AgentToolTrustToggle", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows the untrusted status and requires confirmation before enabling", async () => {
    const setAgentToolTrust = vi.fn(async () => agent);
    const service = createAgentServiceDouble({
      getApiAgentProviderConfig: async () => ({
        modelId: "claude-opus-4-8",
        interfaceFormat: "anthropic",
        baseUrl: null,
        autoApproveTools: false,
      }),
      setAgentToolTrust,
    });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const { user } = renderWithAppProviders(<AgentToolTrustToggle agent={agent} service={service} />);

    expect(await screen.findByText("Shell 和文件修改需要审批")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "启用自动批准" }));

    expect(window.confirm).toHaveBeenCalled();
    expect(setAgentToolTrust).toHaveBeenCalledWith("my-api-agent", true);
  });

  it("does not enable when the confirmation is dismissed", async () => {
    const setAgentToolTrust = vi.fn(async () => agent);
    const service = createAgentServiceDouble({
      getApiAgentProviderConfig: async () => ({
        modelId: "claude-opus-4-8",
        interfaceFormat: "anthropic",
        baseUrl: null,
        autoApproveTools: false,
      }),
      setAgentToolTrust,
    });
    vi.spyOn(window, "confirm").mockReturnValue(false);
    const { user } = renderWithAppProviders(<AgentToolTrustToggle agent={agent} service={service} />);

    await screen.findByText("Shell 和文件修改需要审批");
    await user.click(screen.getByRole("button", { name: "启用自动批准" }));

    expect(window.confirm).toHaveBeenCalled();
    expect(setAgentToolTrust).not.toHaveBeenCalled();
  });

  it("disables without confirmation when already trusted", async () => {
    const setAgentToolTrust = vi.fn(async () => agent);
    const service = createAgentServiceDouble({
      getApiAgentProviderConfig: async () => ({
        modelId: "claude-opus-4-8",
        interfaceFormat: "anthropic",
        baseUrl: null,
        autoApproveTools: true,
      }),
      setAgentToolTrust,
    });
    const confirmSpy = vi.spyOn(window, "confirm");
    const { user } = renderWithAppProviders(<AgentToolTrustToggle agent={agent} service={service} />);

    expect(await screen.findByText("Shell 和文件修改无需审批即可执行")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "关闭自动批准" }));

    expect(confirmSpy).not.toHaveBeenCalled();
    await waitFor(() => expect(setAgentToolTrust).toHaveBeenCalledWith("my-api-agent", false));
  });
});
