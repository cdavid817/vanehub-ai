// @vitest-environment jsdom

import { readFileSync } from "node:fs";
import { screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { OnePieceProviderProfile, RetrievalIndexStatus } from "../../../types/agent";
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

const unconfigured = { sourceProfileId: null, embeddingModel: null };
const emptyStatus: RetrievalIndexStatus = { indexed: 0, pending: 0, failed: 0, lastFailureCategory: null };

describe("OnePieceRetrievalSection", () => {
  afterEach(() => vi.restoreAllMocks());

  it("lists only openai-compatible profiles as embedding sources", async () => {
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => unconfigured,
      getRetrievalIndexStatus: async () => emptyStatus,
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection agentId="onepiece" profiles={[anthropicProfile, openAiProfile]} service={service} />,
    );

    const select = await screen.findByRole("combobox", { name: "Embedding 来源" });
    expect(within(select).getByRole("option", { name: "OpenRouter" })).toBeTruthy();
    expect(within(select).queryByRole("option", { name: "Anthropic 主账号" })).toBeNull();
  });

  it("stays visible but not configurable when no openai-compatible profile exists", async () => {
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => unconfigured,
      getRetrievalIndexStatus: async () => emptyStatus,
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection agentId="onepiece" profiles={[anthropicProfile]} service={service} />,
    );

    expect(await screen.findByText(/需要一个 openai-compatible/)).toBeTruthy();
    expect(screen.queryByRole("combobox", { name: "Embedding 来源" })).toBeNull();
    expect(screen.getByText("检索索引配置")).toBeTruthy();
  });

  it("renders indexed, pending, and failed counts", async () => {
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => unconfigured,
      getRetrievalIndexStatus: async () => ({ indexed: 5, pending: 3, failed: 2, lastFailureCategory: null }),
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection agentId="onepiece" profiles={[openAiProfile]} service={service} />,
    );

    expect(await screen.findByText("5")).toBeTruthy();
    expect(screen.getByText("3")).toBeTruthy();
    expect(screen.getByText("2")).toBeTruthy();
  });

  it("shows only the failure category, never raw error text", async () => {
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => unconfigured,
      getRetrievalIndexStatus: async () => ({ indexed: 5, pending: 0, failed: 2, lastFailureCategory: "auth" }),
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection agentId="onepiece" profiles={[openAiProfile]} service={service} />,
    );

    expect(await screen.findByText(/鉴权失败/)).toBeTruthy();
    expect(screen.queryByText("auth")).toBeNull();
  });

  it("requeues everything when rebuild is confirmed", async () => {
    const getRetrievalIndexStatus = vi.fn()
      .mockResolvedValueOnce({ indexed: 5, pending: 0, failed: 3, lastFailureCategory: "network" })
      .mockResolvedValue({ indexed: 0, pending: 8, failed: 0, lastFailureCategory: null });
    const rebuildRetrievalIndex = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => ({ sourceProfileId: openAiProfile.id, embeddingModel: "text-embedding-3-small" }),
      getRetrievalIndexStatus,
      rebuildRetrievalIndex,
      listEmbeddingModels: async () => [],
    });
    const { user } = renderWithAppProviders(
      <OnePieceRetrievalSection agentId="onepiece" profiles={[openAiProfile]} service={service} />,
    );

    await screen.findByText("3");
    await user.click(screen.getByRole("button", { name: "重建索引" }));
    const dialog = await screen.findByRole("dialog", { name: "重建检索索引" });
    await user.click(within(dialog).getByRole("button", { name: "确认重建" }));

    await waitFor(() => expect(rebuildRetrievalIndex).toHaveBeenCalledWith("onepiece"));
    await waitFor(() => expect(getRetrievalIndexStatus).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("8")).toBeTruthy();
  });

  it("loads embedding models through the service boundary, never invoke()", async () => {
    const listEmbeddingModels = vi.fn(async () => [{ id: "text-embedding-3-small", displayName: "text-embedding-3-small" }]);
    const service = createAgentServiceDouble({
      getRetrievalConfiguration: async () => ({ sourceProfileId: openAiProfile.id, embeddingModel: null }),
      getRetrievalIndexStatus: async () => emptyStatus,
      listEmbeddingModels,
    });
    renderWithAppProviders(
      <OnePieceRetrievalSection agentId="onepiece" profiles={[openAiProfile]} service={service} />,
    );

    await waitFor(() => expect(listEmbeddingModels).toHaveBeenCalledWith(openAiProfile.id));
    expect(await screen.findByRole("option", { name: "text-embedding-3-small" })).toBeTruthy();

    const source = readFileSync("src/settings/pages/agents/onepiece-retrieval-section.tsx", "utf8");
    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
  });
});
