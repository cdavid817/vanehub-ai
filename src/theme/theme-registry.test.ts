import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
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

  it("keeps registered styles aligned on visual design tokens", () => {
    const css = readFileSync("src/styles.css", "utf8");
    const requiredTokens = [
      "--panel",
      "--panel-muted",
      "--panel-border",
      "--panel-hover",
      "--panel-glass",
      "--border-strong",
      "--success",
      "--warning",
      "--danger",
      "--shadow-color",
      "--shadow-elevated",
    ];

    for (const theme of ucdThemes) {
      const block = css.match(new RegExp(`:root\\[data-theme="${theme.id}"\\] \\{([\\s\\S]*?)\\n\\}`))?.[1] ?? "";
      for (const token of requiredTokens) {
        expect(block).toContain(token);
      }
    }
  });
});
