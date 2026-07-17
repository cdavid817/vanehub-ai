import { describe, expect, it } from "vitest";
import { settingsPages } from "./settings-pages";

describe("settingsPages", () => {
  it("registers Usage Statistics before About", () => {
    const usageIndex = settingsPages.findIndex((page) => page.id === "usage");
    const aboutIndex = settingsPages.findIndex((page) => page.id === "about");

    expect(usageIndex).toBeGreaterThan(-1);
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
