import { describe, expect, it } from "vitest";
import { cliAgentProviderPresets, getCliConfigPresets } from "./cli-agent-provider-presets";
import { cliConfigAgentIds } from "../types/cli-agent-config";

describe("CLI Agent provider presets", () => {
  it("ships a secret-free compatible catalog for every supported Agent", () => {
    const ids = new Set<string>();
    for (const preset of cliAgentProviderPresets) {
      expect(ids.has(preset.id)).toBe(false);
      ids.add(preset.id);
      expect(cliConfigAgentIds).toContain(preset.agentId);
      const serialized = JSON.stringify(preset).toLowerCase();
      expect(serialized).not.toContain("api_key\":");
      expect(serialized).not.toContain("authorization\":");
      expect(serialized).not.toContain("credentialref");
      expect(serialized).not.toContain("<script");
    }
    for (const agentId of cliConfigAgentIds) {
      // Endpoint-capable Agents get one preset per provider endpoint. Antigravity accepts no
      // third-party endpoint, so a per-provider catalog would be a menu of unusable choices; it
      // ships exactly one official settings preset instead.
      const minimum = agentId === "antigravity-cli" ? 1 : 8;
      expect(getCliConfigPresets(agentId).length).toBeGreaterThanOrEqual(minimum);
      expect(getCliConfigPresets(agentId).every((preset) => preset.agentId === agentId)).toBe(true);
    }
  });

  it("offers no endpoint or credential surface for Antigravity", () => {
    const presets = getCliConfigPresets("antigravity-cli");
    expect(presets).toHaveLength(1);
    for (const preset of presets) {
      expect(preset.payload.kind).toBe("antigravity");
      expect(preset.payload).not.toHaveProperty("baseUrl");
      expect(preset.payload).not.toHaveProperty("authMode");
      expect(preset.payload).not.toHaveProperty("authStrategy");
    }
  });

  it("projects only compatible endpoint types from the shared directory", () => {
    expect(getCliConfigPresets("claude-code").every((preset) => preset.endpointType === "anthropic-messages")).toBe(true);
    expect(getCliConfigPresets("codex-cli").every((preset) => preset.endpointType !== "anthropic-messages")).toBe(true);
    expect(getCliConfigPresets("opencode").every((preset) => preset.endpointType !== "anthropic-messages")).toBe(true);
    // 25 from the shared endpoint directory, plus Antigravity's own single non-directory provider.
    expect(new Set(cliAgentProviderPresets.map((preset) => preset.providerId)).size).toBe(26);
  });

  it("returns editable copies instead of mutating the catalog", () => {
    const first = getCliConfigPresets("claude-code");
    const payload = first[0]?.payload;
    if (payload?.kind !== "claude-code") throw new Error("fixture mismatch");
    payload.model = "changed";
    const second = getCliConfigPresets("claude-code");
    expect(second[0]?.payload.kind === "claude-code" && second[0].payload.model).not.toBe("changed");
  });
});
