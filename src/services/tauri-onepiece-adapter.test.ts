import { beforeEach, describe, expect, it, vi } from "vitest";
import type { OnePieceProviderProfiles, SaveOnePieceProviderProfileInput } from "../types/agent";

const { invokeMock, openUrlMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  openUrlMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

import { tauriAgentClient } from "./tauri-agent-client";

describe("Tauri OnePiece adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ profiles: [], activeProfileId: null } satisfies OnePieceProviderProfiles);
    openUrlMock.mockReset();
    openUrlMock.mockResolvedValue(undefined);
  });

  it("opens HTTPS provider links through the Tauri opener plugin", async () => {
    await tauriAgentClient.openExternalUrl("https://console.anthropic.com/settings/keys");

    expect(openUrlMock).toHaveBeenCalledWith("https://console.anthropic.com/settings/keys");
    await expect(tauriAgentClient.openExternalUrl("javascript:alert(1)"))
      .rejects.toThrow("Only HTTPS external URLs are allowed.");
    expect(openUrlMock).toHaveBeenCalledTimes(1);
  });

  it("keeps profile list, save, activate, and delete as thin command wrappers", async () => {
    const input: SaveOnePieceProviderProfileInput = {
      id: null,
      name: "Anthropic 主账号",
      providerId: "anthropic",
      endpointType: "anthropic-messages",
      modelId: "claude-test",
      apiKey: "sk-input-only",
    };

    await tauriAgentClient.listOnePieceProviderProfiles();
    await tauriAgentClient.listOnePieceProviderPresets();
    await tauriAgentClient.discoverOnePieceProviderModels({ providerId: "anthropic", endpointType: "anthropic-messages", profileId: "profile-1", apiKey: null });
    await tauriAgentClient.validateOnePieceProviderCredential({ providerId: "anthropic", endpointType: "anthropic-messages", modelId: "claude-test", profileId: "profile-1", apiKey: null });
    await tauriAgentClient.saveOnePieceProviderProfile(input);
    await tauriAgentClient.activateOnePieceProviderProfile("profile-1");
    await tauriAgentClient.deleteOnePieceProviderProfile("profile-1");

    expect(invokeMock.mock.calls).toEqual([
      ["list_onepiece_provider_profiles"],
      ["list_onepiece_provider_presets"],
      ["discover_onepiece_provider_models", { input: { providerId: "anthropic", endpointType: "anthropic-messages", profileId: "profile-1", apiKey: null } }],
      ["validate_onepiece_provider_credential", { input: { providerId: "anthropic", endpointType: "anthropic-messages", modelId: "claude-test", profileId: "profile-1", apiKey: null } }],
      ["save_onepiece_provider_profile", { input }],
      ["activate_onepiece_provider_profile", { profileId: "profile-1" }],
      ["delete_onepiece_provider_profile", { profileId: "profile-1" }],
    ]);
  });
});
