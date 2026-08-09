// @vitest-environment jsdom

import { readFileSync } from "node:fs";
import { screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { OnePieceProviderProfile } from "../../../types/agent";
import { OnePieceRetrievalSection } from "./onepiece-retrieval-section";

const anthropicProfile: OnePieceProviderProfile = {
  id: "anthropic-primary",
  name: "Anthropic 主账号",
  sourceProviderId: "anthropic",
  sourceEndpointType: "anthropic-messages",
  sourcePresetVersion: 1,
  provider: "Anthropic",
  modelId: "claude-test",
  interfaceFormat: "anthropic",
  baseUrl: null,
  active: true,
  credentialPresent: true,
};

const openAiProfile: OnePieceProviderProfile = {
  id: "openrouter",
  name: "OpenRouter",
  sourceProviderId: "openrouter",
  sourceEndpointType: "openai-chat-completions",
  sourcePresetVersion: 1,
  provider: "OpenRouter",
  modelId: "gpt-test",
  interfaceFormat: "openai-compatible",
  baseUrl: "https://openrouter.ai/api/v1",
  active: false,
  credentialPresent: true,
};

const unconfigured = { sourceProfileId: null, embeddingModel: null, automaticCodeIndexMode: "semantic" as const };

describe("OnePieceRetrievalSection", () => {
  afterEach(() => vi.restoreAllMocks());

  it("saves local automatic indexing without requiring an embedding model", async () => {
    const saveCodeIndexAutomaticMode = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => ({
        sourceProfileId: null,
        embeddingModel: null,
        automaticCodeIndexMode: "disabled",
      }),
      saveCodeIndexAutomaticMode,
    });
    const { user } = renderWithAppProviders(
      <OnePieceRetrievalSection profiles={[openAiProfile]} service={service} />,
    );

    await user.selectOptions(await screen.findByRole("combobox", { name: "自动项目代码索引" }), "local");
    await waitFor(() => expect(saveCodeIndexAutomaticMode).toHaveBeenCalledWith("local"));
    expect(screen.queryByRole("combobox", { name: "Embedding 来源" })).toBeNull();
  });

  it("lists only openai-compatible profiles as embedding sources", async () => {
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => unconfigured,
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection profiles={[anthropicProfile, openAiProfile]} service={service} />,
    );

    const select = await screen.findByRole("combobox", { name: "Embedding 来源" });
    expect(within(select).getByRole("option", { name: "OpenRouter" })).toBeTruthy();
    expect(within(select).queryByRole("option", { name: "Anthropic 主账号" })).toBeNull();
  });

  it("stays visible but not configurable when no openai-compatible profile exists", async () => {
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => unconfigured,
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection profiles={[anthropicProfile]} service={service} />,
    );

    expect(await screen.findByText(/需要一个 openai-compatible/)).toBeTruthy();
    expect(screen.queryByRole("combobox", { name: "Embedding 来源" })).toBeNull();
    expect(screen.getByText("检索索引配置")).toBeTruthy();
  });

  it("keeps index status and rebuild controls out of parameter management", async () => {
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => unconfigured,
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection profiles={[openAiProfile]} service={service} />,
    );

    await screen.findByText("检索索引配置");
    expect(screen.queryByText("索引状态")).toBeNull();
    expect(screen.queryByRole("button", { name: "重建索引" })).toBeNull();
    expect(screen.getAllByRole("combobox")[0].className).toContain("min-h-9");
  });

  it("loads embedding models through the service boundary, never invoke()", async () => {
    const listEmbeddingModels = vi.fn(async () => [{ id: "text-embedding-3-small", displayName: "text-embedding-3-small" }]);
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => ({ sourceProfileId: openAiProfile.id, embeddingModel: null, automaticCodeIndexMode: "semantic" }),
      listEmbeddingModels,
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection profiles={[openAiProfile]} service={service} />,
    );

    await waitFor(() => expect(listEmbeddingModels).toHaveBeenCalledWith(openAiProfile.id));
    expect(await screen.findByRole("option", { name: "text-embedding-3-small" })).toBeTruthy();

    const source = readFileSync("src/settings/pages/agents/onepiece-retrieval-section.tsx", "utf8");
    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
  });
});
