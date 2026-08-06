// @vitest-environment jsdom

import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { getOnePieceProviderPresets } from "../../../config/onepiece-provider-presets";
import type { AgentService } from "../../../services/agent-service";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { OnePieceProviderProfile, OnePieceProviderProfiles } from "../../../types/agent";
import { OnePieceConfigurationPanel } from "./onepiece-configuration-panel";

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

const proxyProfile: OnePieceProviderProfile = {
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

function overview(profiles: OnePieceProviderProfile[], activeProfileId: string | null): OnePieceProviderProfiles {
  return { profiles, activeProfileId };
}

// The panel unconditionally mounts OnePieceRetrievalSection (onepiece-configuration-panel.tsx),
// which fires these two queries on every render. Centralized here so every double in this file
// gets harmless defaults without repeating them per test; a future third retrieval method needs
// one edit here, not one per `it` block. Individual tests still override either key when the
// scenario under test needs to observe or control that call.
const retrievalDoubleDefaults: Partial<AgentService> = {
  getRetrievalConfiguration: async () => ({ sourceProfileId: null, embeddingModel: null }),
  getRetrievalIndexStatus: async () => ({ indexed: 0, pending: 0, failed: 0, lastFailureCategory: null }),
};

function createPanelServiceDouble(overrides: Partial<AgentService>): AgentService {
  return createAgentServiceDouble({ ...retrievalDoubleDefaults, ...overrides });
}

describe("OnePieceConfigurationPanel", () => {
  afterEach(() => vi.restoreAllMocks());

  const listOnePieceProviderPresets = async () => getOnePieceProviderPresets();

  it("verifies a saved OnePiece credential from its provider card", async () => {
    const validateOnePieceProviderCredential = vi.fn(async () => ({ status: "invalid-credential" as const, latencyMs: 18, httpStatus: 401 }));
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => overview([anthropicProfile], anthropicProfile.id),
      listOnePieceProviderPresets,
      validateOnePieceProviderCredential,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    const card = (await screen.findByRole("heading", { name: "Anthropic 主账号" })).closest("article");
    expect(card).not.toBeNull();
    await user.click(within(card as HTMLElement).getByRole("button", { name: "验证 API 密钥" }));
    await waitFor(() => expect(validateOnePieceProviderCredential).toHaveBeenCalledWith({
      providerId: "anthropic",
      endpointType: "anthropic-messages",
      modelId: "claude-test",
      profileId: "anthropic-primary",
      apiKey: null,
    }));
    expect(await within(card as HTMLElement).findByText("API 密钥被厂商拒绝。")).toBeTruthy();
  });

  it("adds the first named provider Profile and keeps its secret write-only", async () => {
    const configured = overview([anthropicProfile], anthropicProfile.id);
    const saveOnePieceProviderProfile = vi.fn(async () => configured);
    const onChanged = vi.fn(async () => undefined);
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => overview([], null),
      listOnePieceProviderPresets,
      saveOnePieceProviderProfile,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={onChanged} service={service} />,
    );

    expect(await screen.findByText("需要完成配置并保存提供商凭证")).toBeTruthy();
    await user.click(screen.getAllByRole("button", { name: "新增配置" })[0]);
    const dialog = screen.getByRole("dialog", { name: "新增 OnePiece 配置" });
    await user.click(within(dialog).getByRole("button", { name: /Anthropic/ }));
    fireEvent.change(within(dialog).getByLabelText("配置名称"), { target: { value: "Anthropic 主账号" } });
    fireEvent.change(within(dialog).getByLabelText("模型"), { target: { value: "claude-opus-4-6" } });
    fireEvent.change(within(dialog).getByLabelText("API 密钥"), { target: { value: "sk-input-only" } });
    await user.click(within(dialog).getByRole("button", { name: "保存 OnePiece" }));

    await waitFor(() => expect(saveOnePieceProviderProfile).toHaveBeenCalledWith({
      id: null,
      name: "Anthropic 主账号",
      providerId: "anthropic",
      endpointType: "anthropic-messages" as const,
      modelId: "claude-opus-4-6",
      apiKey: "sk-input-only",
    }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("heading", { name: "Anthropic 主账号" })).toBeTruthy();
    expect(screen.getByText("当前使用")).toBeTruthy();
    expect(screen.queryByDisplayValue("sk-input-only")).toBeNull();
    expect(onChanged).toHaveBeenCalledOnce();
  });

  it("shows multiple Profiles and requires app-owned confirmation before activation", async () => {
    const initial = overview([anthropicProfile, proxyProfile], anthropicProfile.id);
    const activatedProxy = { ...proxyProfile, active: true };
    const activated = overview([{ ...anthropicProfile, active: false }, activatedProxy], proxyProfile.id);
    const activateOnePieceProviderProfile = vi.fn(async () => activated);
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => initial,
      listOnePieceProviderPresets,
      activateOnePieceProviderProfile,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    await screen.findByRole("heading", { name: "Anthropic 主账号" });
    expect(screen.getByRole("heading", { name: "OpenRouter" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "启用" }));
    const dialog = screen.getByRole("dialog", { name: "启用提供商" });
    expect(within(dialog).getByText(/OpenRouter/)).toBeTruthy();
    expect(activateOnePieceProviderProfile).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "确认启用" }));

    await waitFor(() => expect(activateOnePieceProviderProfile).toHaveBeenCalledWith(proxyProfile.id));
    expect(await screen.findByText("OnePiece 提供商已启用。")).toBeTruthy();
  });

  it("filters provider cards with the Settings search term", async () => {
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => overview([anthropicProfile, proxyProfile], anthropicProfile.id),
      listOnePieceProviderPresets,
    });
    renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} searchTerm="gpt-test" service={service} />,
    );

    expect(await screen.findByRole("heading", { name: "OpenRouter" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Anthropic 主账号" })).toBeNull();
  });

  it("warns when deleting the active Profile and does not select a fallback", async () => {
    const initial = overview([anthropicProfile, proxyProfile], anthropicProfile.id);
    const deleted = overview([proxyProfile], null);
    const deleteOnePieceProviderProfile = vi.fn(async () => deleted);
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => initial,
      listOnePieceProviderPresets,
      deleteOnePieceProviderProfile,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    await screen.findByRole("heading", { name: "Anthropic 主账号" });
    const activeCard = screen.getByRole("heading", { name: "Anthropic 主账号" }).closest("article");
    expect(activeCard).not.toBeNull();
    await user.click(within(activeCard as HTMLElement).getByRole("button", { name: "删除" }));
    const dialog = screen.getByRole("dialog", { name: "删除提供商" });
    expect(within(dialog).getByText(/不会自动切换到其他配置/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "确认删除" }));

    await waitFor(() => expect(deleteOnePieceProviderProfile).toHaveBeenCalledWith(anthropicProfile.id));
    expect(await screen.findByText("需要完成配置并保存提供商凭证")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "OpenRouter" })).toBeTruthy();
    expect(screen.getByText("未启用")).toBeTruthy();
  });

  it("edits a Profile without repopulating or replacing its stored credential", async () => {
    const initial = overview([anthropicProfile], anthropicProfile.id);
    const updatedProfile = { ...anthropicProfile, modelId: "claude-opus-4-6" };
    const saveOnePieceProviderProfile = vi.fn(async () => overview([updatedProfile], updatedProfile.id));
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => initial,
      listOnePieceProviderPresets,
      saveOnePieceProviderProfile,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    await screen.findByRole("heading", { name: "Anthropic 主账号" });
    await user.click(screen.getByRole("button", { name: "编辑配置" }));
    const dialog = screen.getByRole("dialog", { name: "编辑 OnePiece 配置" });
    const keyInput = within(dialog).getByLabelText("API 密钥") as HTMLInputElement;
    expect(keyInput.value).toBe("");
    expect(keyInput.placeholder).toBe("留空以保留已存凭证");
    fireEvent.change(within(dialog).getByLabelText("模型"), { target: { value: "claude-opus-4-6" } });
    await user.click(within(dialog).getByRole("button", { name: "保存 OnePiece" }));

    await waitFor(() => expect(saveOnePieceProviderProfile).toHaveBeenCalledWith({
      id: anthropicProfile.id,
      name: "Anthropic 主账号",
      providerId: "anthropic",
      endpointType: "anthropic-messages" as const,
      modelId: "claude-opus-4-6",
      apiKey: null,
    }));
  });

  it("fetches provider models with the stored Profile credential and selects an API result", async () => {
    const discoverOnePieceProviderModels = vi.fn(async () => ({
      providerId: "anthropic",
      endpointType: "anthropic-messages" as const,
      models: [{ id: "claude-live", displayName: "Claude Live", source: "api" as const }],
      source: "merged" as const,
      warning: null,
    }));
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => overview([anthropicProfile], anthropicProfile.id),
      listOnePieceProviderPresets,
      discoverOnePieceProviderModels,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    await screen.findByRole("heading", { name: "Anthropic 主账号" });
    await user.click(screen.getByRole("button", { name: "编辑配置" }));
    const dialog = screen.getByRole("dialog", { name: "编辑 OnePiece 配置" });
    await user.click(within(dialog).getByRole("button", { name: "获取模型" }));

    await waitFor(() => expect(discoverOnePieceProviderModels).toHaveBeenCalledWith({
      providerId: "anthropic",
      endpointType: "anthropic-messages",
      profileId: anthropicProfile.id,
      apiKey: null,
    }));
    expect(await within(dialog).findByRole("option", { name: "Claude Live (claude-live)" })).toBeTruthy();
  });

  it("ignores a stale model response after the credential changes", async () => {
    let resolveDiscovery: ((value: {
      providerId: string;
      endpointType: "anthropic-messages";
      models: Array<{ id: string; displayName: string; source: "api" }>;
      source: "merged";
      warning: null;
    }) => void) | undefined;
    const discoverOnePieceProviderModels = vi.fn(() => new Promise<{
      providerId: string;
      endpointType: "anthropic-messages";
      models: Array<{ id: string; displayName: string; source: "api" }>;
      source: "merged";
      warning: null;
    }>((resolve) => { resolveDiscovery = resolve; }));
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => overview([anthropicProfile], anthropicProfile.id),
      listOnePieceProviderPresets,
      discoverOnePieceProviderModels,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    await screen.findByRole("heading", { name: "Anthropic 主账号" });
    await user.click(screen.getByRole("button", { name: "编辑配置" }));
    const dialog = screen.getByRole("dialog", { name: "编辑 OnePiece 配置" });
    await user.click(within(dialog).getByRole("button", { name: "获取模型" }));
    fireEvent.change(within(dialog).getByLabelText("API 密钥"), { target: { value: "replacement" } });
    resolveDiscovery?.({
      providerId: "anthropic",
      endpointType: "anthropic-messages",
      models: [{ id: "stale-model", displayName: "Stale Model", source: "api" }],
      source: "merged",
      warning: null,
    });

    await waitFor(() => expect(within(dialog).getByRole("button", { name: "获取模型" })).toBeTruthy());
    expect(within(dialog).queryByRole("option", { name: /Stale Model/ })).toBeNull();
  });

  it("shows the catalog fallback warning and allows retry", async () => {
    const discoverOnePieceProviderModels = vi.fn(async () => ({
      providerId: "anthropic",
      endpointType: "anthropic-messages" as const,
      models: [{ id: "claude-sonnet-4-6", displayName: "claude-sonnet-4-6", source: "catalog" as const }],
      source: "catalog" as const,
      warning: "live-unavailable" as const,
    }));
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => overview([anthropicProfile], anthropicProfile.id),
      listOnePieceProviderPresets,
      discoverOnePieceProviderModels,
    });
    const { user } = renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    await screen.findByRole("heading", { name: "Anthropic 主账号" });
    await user.click(screen.getByRole("button", { name: "编辑配置" }));
    const dialog = screen.getByRole("dialog", { name: "编辑 OnePiece 配置" });
    await user.click(within(dialog).getByRole("button", { name: "获取模型" }));
    expect(await within(dialog).findByText(/暂时无法从厂商获取模型/)).toBeTruthy();
    expect((within(dialog).getByRole("button", { name: "获取模型" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("mounts the retrieval section with the panel's own Profile list", async () => {
    const getRetrievalIndexStatus = vi.fn(async () => ({ indexed: 0, pending: 0, failed: 0, lastFailureCategory: null }));
    const service = createPanelServiceDouble({
      listOnePieceProviderProfiles: async () => overview([anthropicProfile, proxyProfile], anthropicProfile.id),
      listOnePieceProviderPresets,
      getRetrievalIndexStatus,
    });
    renderWithAppProviders(
      <OnePieceConfigurationPanel onChanged={vi.fn(async () => undefined)} service={service} />,
    );

    await screen.findByRole("heading", { name: "Anthropic 主账号" });
    // Index status is global, like the configuration singleton it sits next to — the section takes
    // no agent id, so the only thing to pin here is that the panel really mounts it and it really
    // queries through the service boundary.
    await waitFor(() => expect(getRetrievalIndexStatus).toHaveBeenCalledWith());

    // Pins the `profiles={overview.profiles}` argument: the openai-compatible Profile the panel
    // loaded is the one the section offers as an embedding source, and the Anthropic Profile
    // (no embeddings API) is filtered out — proof this is the panel's real data, not a stub.
    const select = await screen.findByRole("combobox", { name: "Embedding 来源" });
    expect(within(select).getByRole("option", { name: "OpenRouter" })).toBeTruthy();
    expect(within(select).queryByRole("option", { name: "Anthropic 主账号" })).toBeNull();

    expect(screen.queryByRole("alert")).toBeNull();
  });
});
