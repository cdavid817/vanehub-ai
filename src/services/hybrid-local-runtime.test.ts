import { beforeEach, describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";

describe("Web hybrid local runtime", () => {
  beforeEach(async () => {
    await webAgentClient.resetOnePieceProviderConfig();
  });

  it("simulates discovery without network and preserves configured provenance", async () => {
    const discovery = await webAgentClient.discoverLocalModelEndpoints();
    expect(discovery.operationId).toBe("web-local-discovery-simulated");
    const overview = await webAgentClient.saveCustomOnePieceProviderProfile({
      name: "Local Qwen",
      baseUrl: "http://127.0.0.1:11434/v1",
      modelId: discovery.candidates[0]?.models[0] ?? "qwen",
      runtimeKind: "local",
      authenticationMode: "none",
      timeoutMs: 30_000,
      privacyClassification: "local",
      toolCallingCapability: "unsupported",
      imageInputCapability: "unknown",
      structuredOutputCapability: "unknown",
      reasoningFieldCapability: "unknown",
      contextWindowTokens: 32_768,
      reservedOutputTokens: 4_096,
    });
    const profile = overview.profiles[0];
    expect(profile?.provider).toBe("Local endpoint");
    expect(profile?.credentialPresent).toBe(false);
    const metadata = await webAgentClient.getEndpointProfileMetadata(profile?.id ?? "");
    expect(metadata).toMatchObject({ runtimeKind: "local", capabilityProvenance: "configured" });
  });

  it("blocks cloud fallback under local-only and rejects unsupported tools", async () => {
    const local = await webAgentClient.saveCustomOnePieceProviderProfile({
      name: "Local text",
      baseUrl: "http://localhost:11434/v1",
      modelId: "text-only",
      runtimeKind: "local",
      authenticationMode: "none",
      timeoutMs: 30_000,
      privacyClassification: "local",
      toolCallingCapability: "unsupported",
      imageInputCapability: "unknown",
      structuredOutputCapability: "unknown",
      reasoningFieldCapability: "unknown",
      contextWindowTokens: null,
      reservedOutputTokens: 0,
    });
    const profileId = local.profiles[0]?.id ?? "";
    await webAgentClient.replaceHybridRoutingRules([{ id: "summary", enabled: true, orderIndex: 0, taskClass: "summarization", preferredProfileId: profileId, fallbackProfileId: null, dataPolicy: "local-only" }]);
    const preview = await webAgentClient.previewHybridRoute({ taskClass: "summarization", dataPolicy: "local-only", activeProfileId: profileId, hybridEnabled: true, requiresTools: true, requiresImageInput: false, requiresStructuredOutput: false, requestsReasoningField: false });
    expect(preview).toMatchObject({ profileId: null, waitingForUserChoice: true, reason: "waiting-local-only" });
  });
});
