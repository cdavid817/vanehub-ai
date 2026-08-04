// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProviderCatalog } from "./provider-catalog";
import { ProviderEndpointSelector, ProviderHelpLinks } from "./provider-endpoint-controls";
import { CliConfigProviderCatalog } from "../../settings/pages/agents/cli-config-provider-catalog";
import { getCliConfigPresets } from "../../config/cli-agent-provider-presets";

describe("shared provider directory components", () => {
  it("filters shared provider cards by category and search", () => {
    render(<ProviderCatalog description="Directory" emptyLabel="Empty" items={[
      { id: "anthropic", displayName: "Anthropic", category: "official", iconKey: "anthropic", catalogVersion: 3, searchText: "Claude" },
      { id: "deepseek", displayName: "DeepSeek", category: "common", iconKey: "deepseek", catalogVersion: 3, searchText: "deepseek-chat" },
    ]} onSelect={() => undefined} searchLabel="Search providers" selectedId={null} title="Providers" />);
    fireEvent.change(screen.getByLabelText("Search providers"), { target: { value: "deepseek" } });
    expect(screen.queryByRole("button", { name: "Anthropic" })).toBeNull();
    expect(screen.getByRole("button", { name: "DeepSeek" })).toBeTruthy();
  });

  it("selects an explicit endpoint without exposing an editable URL", () => {
    const onChange = vi.fn();
    render(<ProviderEndpointSelector endpoints={[
      { type: "anthropic-messages", baseUrl: "https://example.test/anthropic" },
      { type: "openai-chat-completions", baseUrl: "https://example.test/v1" },
    ]} label="API endpoint" onChange={onChange} value="anthropic-messages" />);
    fireEvent.click(screen.getByRole("button", { name: /OpenAI Chat Completions/ }));
    expect(onChange).toHaveBeenCalledWith("openai-chat-completions");
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("renders one CLI card per vendor and selects the preferred Codex endpoint", () => {
    const onSelect = vi.fn();
    const presets = getCliConfigPresets("codex-cli");
    render(<CliConfigProviderCatalog onCreateCustom={() => undefined} onSelectPreset={onSelect} presets={presets} selectedPresetId={null} />);
    expect(screen.getAllByRole("button", { name: "OpenRouter" })).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "OpenRouter" }));
    expect(onSelect.mock.calls[0]?.[0]).toMatchObject({ providerId: "openrouter", endpointType: "openai-responses" });
  });

  it("routes provider help links through the application opener", () => {
    const onOpenUrl = vi.fn();
    render(<ProviderHelpLinks apiKeyLabel="Open API key page" apiKeyUrl="https://example.test/keys" docsLabel="View docs" docsUrl="https://example.test/docs" onOpenUrl={onOpenUrl} />);
    fireEvent.click(screen.getByRole("link", { name: /Open API key page/ }));
    expect(onOpenUrl).toHaveBeenCalledWith("https://example.test/keys");
  });
});
