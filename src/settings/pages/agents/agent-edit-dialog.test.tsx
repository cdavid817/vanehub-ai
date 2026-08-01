// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { AgentRegistryEntry } from "../../../types/agent";
import { AgentEditDialog } from "./agent-edit-dialog";

const agent: AgentRegistryEntry = {
  id: "my-api-agent",
  displayName: "My API Agent",
  provider: "Anthropic",
  launch: { kind: "api" },
  supportedInteractionModes: ["api"],
  availabilityState: "available",
  capabilityTags: ["api"],
};

describe("AgentEditDialog", () => {
  afterEach(() => vi.restoreAllMocks());

  it("pre-fills the form with the agent's current provider config", async () => {
    const service = createAgentServiceDouble({
      getApiAgentProviderConfig: async () => ({
        modelId: "claude-opus-4-8",
        interfaceFormat: "anthropic",
        baseUrl: null,
        autoApproveTools: false,
      }),
    });
    renderWithAppProviders(<AgentEditDialog agent={agent} onClose={() => undefined} onSaved={() => undefined} service={service} />);

    expect(await screen.findByDisplayValue("claude-opus-4-8")).toBeTruthy();
    expect(screen.getByDisplayValue("My API Agent")).toBeTruthy();
  });

  it("saves the edited fields and calls onSaved with the updated agent", async () => {
    const updated: AgentRegistryEntry = { ...agent, displayName: "Renamed Agent" };
    const updateApiAgent = vi.fn(async () => updated);
    const service = createAgentServiceDouble({
      getApiAgentProviderConfig: async () => ({
        modelId: "claude-opus-4-8",
        interfaceFormat: "anthropic",
        baseUrl: null,
        autoApproveTools: false,
      }),
      updateApiAgent,
    });
    const onSaved = vi.fn();
    const { user } = renderWithAppProviders(
      <AgentEditDialog agent={agent} onClose={() => undefined} onSaved={onSaved} service={service} />,
    );

    await screen.findByDisplayValue("claude-opus-4-8");
    const nameInput = screen.getByDisplayValue("My API Agent");
    await user.clear(nameInput);
    await user.type(nameInput, "Renamed Agent");
    await user.click(screen.getByRole("button", { name: "保存修改" }));

    await waitFor(() => expect(updateApiAgent).toHaveBeenCalledWith("my-api-agent", {
      displayName: "Renamed Agent",
      modelId: "claude-opus-4-8",
      baseUrl: null,
      newApiKey: null,
    }));
    expect(onSaved).toHaveBeenCalledWith(updated);
  });

  it("shows the server error inline without closing when the update is rejected", async () => {
    const service = createAgentServiceDouble({
      getApiAgentProviderConfig: async () => ({
        modelId: "claude-opus-4-8",
        interfaceFormat: "anthropic",
        baseUrl: null,
        autoApproveTools: false,
      }),
      updateApiAgent: async () => {
        throw new Error("Cannot update: agent not found.");
      },
    });
    const onSaved = vi.fn();
    const { user } = renderWithAppProviders(
      <AgentEditDialog agent={agent} onClose={() => undefined} onSaved={onSaved} service={service} />,
    );

    await screen.findByDisplayValue("claude-opus-4-8");
    await user.click(screen.getByRole("button", { name: "保存修改" }));

    expect(await screen.findByText("Cannot update: agent not found.")).toBeTruthy();
    expect(onSaved).not.toHaveBeenCalled();
  });
});
