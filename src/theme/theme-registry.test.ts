import { describe, expect, it } from "vitest";
import { defaultThemeId, getNextThemeId, getThemeDefinition, isUcdThemeId, normalizeThemeId, ucdThemes } from "./theme-registry";

describe("theme registry", () => {
  it("registers the UCD styles used by the style switcher", () => {
    expect(ucdThemes.map((theme) => theme.id)).toEqual(["futuristic", "minimal"]);
  });

  it("validates and normalizes persisted theme ids", () => {
    expect(isUcdThemeId("minimal")).toBe(true);
    expect(isUcdThemeId("unknown")).toBe(false);
    expect(normalizeThemeId("minimal")).toBe("minimal");
    expect(normalizeThemeId("unknown")).toBe(defaultThemeId);
  });

  it("derives display names and next theme from the registry", () => {
    expect(getThemeDefinition("futuristic").displayName).toBe("Futuristic");
    expect(getNextThemeId("futuristic")).toBe("minimal");
    expect(getNextThemeId("minimal")).toBe("futuristic");
  });
});
