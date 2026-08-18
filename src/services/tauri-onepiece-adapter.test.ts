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

  it("maps hybrid local runtime methods only through Tauri commands", async () => {
    const input = {
      name: "Local model",
      baseUrl: "http://127.0.0.1:11434/v1",
      modelId: "qwen",
      runtimeKind: "local" as const,
      authenticationMode: "none" as const,
      timeoutMs: 30_000,
      privacyClassification: "local" as const,
      toolCallingCapability: "unknown" as const,
      imageInputCapability: "unknown" as const,
      structuredOutputCapability: "unknown" as const,
      reasoningFieldCapability: "unknown" as const,
      contextWindowTokens: 32_768,
      reservedOutputTokens: 4_096,
    };
    const rules = [{ id: "summary", enabled: true, orderIndex: 0, taskClass: "summarization" as const, preferredProfileId: "local", fallbackProfileId: null, dataPolicy: "local-only" as const }];
    const preview = { taskClass: "summarization" as const, dataPolicy: "local-only" as const, activeProfileId: "local", hybridEnabled: true, requiresTools: false, requiresImageInput: false, requiresStructuredOutput: false, requestsReasoningField: false };

    await tauriAgentClient.saveCustomOnePieceProviderProfile(input);
    await tauriAgentClient.getEndpointProfileMetadata("local");
    await tauriAgentClient.discoverLocalModelEndpoints();
    await tauriAgentClient.verifyLocalModelEndpoint("http://127.0.0.1:11434", 1_000);
    await tauriAgentClient.listHybridRoutingRules();
    await tauriAgentClient.replaceHybridRoutingRules(rules);
    await tauriAgentClient.previewHybridRoute(preview);

    expect(invokeMock.mock.calls).toEqual([
      ["save_custom_onepiece_provider_profile", { input }],
      ["get_endpoint_profile_metadata", { profileId: "local" }],
      ["discover_local_model_endpoints"],
      ["verify_local_model_endpoint", { input: { baseUrl: "http://127.0.0.1:11434", timeoutMs: 1_000 } }],
      ["list_hybrid_routing_rules"],
      ["replace_hybrid_routing_rules", { rules }],
      ["preview_hybrid_route", { input: preview }],
    ]);
  });
});
