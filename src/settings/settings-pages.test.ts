import { describe, expect, it } from "vitest";
import { settingsPages } from "./settings-pages";

describe("settingsPages", () => {
  it("hides SDK dependencies from primary navigation", () => {
    expect(settingsPages.map((page) => page.id as string)).not.toContain("sdk");
  });

  it("registers extensions below higher-frequency management pages", () => {
    const imIndex = settingsPages.findIndex((page) => page.id === "im");
    const extensionsIndex = settingsPages.findIndex((page) => page.id === "extensions");

    expect(imIndex).toBeGreaterThan(-1);
    expect(extensionsIndex).toBe(imIndex + 1);
    expect(settingsPages[imIndex]).toMatchObject({
      labelKey: "settings.pages.im",
      searchPlaceholderKey: "settings.search.im",
      badge: 5,
    });
  });

  it("registers Usage Statistics before About", () => {
    const extensionsIndex = settingsPages.findIndex((page) => page.id === "extensions");
    const usageIndex = settingsPages.findIndex((page) => page.id === "usage");
    const aboutIndex = settingsPages.findIndex((page) => page.id === "about");

    expect(usageIndex).toBeGreaterThan(-1);
    expect(extensionsIndex).toBe(usageIndex - 1);
    expect(usageIndex).toBe(aboutIndex - 1);
    expect(settingsPages[usageIndex]).toMatchObject({
      labelKey: "settings.pages.usage",
      searchPlaceholderKey: "settings.search.usage",
    });
  });

  it("registers About as the final settings page", () => {
    expect(settingsPages.at(-1)).toMatchObject({
      id: "about",
      labelKey: "settings.pages.about",
      searchPlaceholderKey: "settings.search.about",
    });
  });
});
