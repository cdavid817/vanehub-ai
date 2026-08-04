import { describe, expect, it } from "vitest";
import { getOnePieceProviderPresets, getSharedProviderCatalog, resolveOnePieceProviderPreset } from "./onepiece-provider-presets";

describe("OnePiece provider catalog", () => {
  it("has stable unique ids, safe links, icons, and fallback models", () => {
    const presets = getOnePieceProviderPresets();
    expect(presets.length).toBeGreaterThanOrEqual(20);
    expect(new Set(presets.map((preset) => preset.id)).size).toBe(presets.length);
    expect(presets.every((preset) => preset.apiKeyUrl.startsWith("https://"))).toBe(true);
    expect(presets.every((preset) => preset.docsUrl.startsWith("https://"))).toBe(true);
    expect(presets.every((preset) => preset.iconKey.length > 0)).toBe(true);
    expect(presets.every((preset) => preset.fallbackModels.includes(preset.defaultModelId))).toBe(true);
  });

  it("contains exactly 25 vendors with explicit evidence-backed endpoint records", () => {
    const presets = getOnePieceProviderPresets();
    expect(presets).toHaveLength(25);
    expect(getSharedProviderCatalog().sourceRevisions).toEqual({
      cherryStudio: "03d266e0299d4ca44ce88f5cc6df398922c83147",
      ccSwitch: "f6e37ed99443890a865669e28bf1caf5e85d466d",
    });
    for (const preset of presets) {
      expect(preset.endpoints.length).toBeGreaterThan(0);
      expect(preset.endpoints.every((endpoint) => endpoint.baseUrl.startsWith("https://") && endpoint.source.length > 0)).toBe(true);
    }
    expect(resolveOnePieceProviderPreset("deepseek", "anthropic-messages")).toMatchObject({
      interfaceFormat: "anthropic",
      baseUrl: "https://api.deepseek.com/anthropic",
    });
  });

  it("returns detached arrays so consumers cannot mutate the shared catalog", () => {
    const first = getOnePieceProviderPresets();
    first[0].fallbackModels.push("mutated-model");
    expect(getOnePieceProviderPresets()[0].fallbackModels).not.toContain("mutated-model");
  });
});
