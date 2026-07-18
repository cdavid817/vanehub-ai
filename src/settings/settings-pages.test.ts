import { describe, expect, it } from "vitest";
import { settingsPages } from "./settings-pages";

describe("settingsPages", () => {
  it("registers IM before Usage Statistics and About", () => {
    const imIndex = settingsPages.findIndex((page) => page.id === "im");
    const usageIndex = settingsPages.findIndex((page) => page.id === "usage");

    expect(imIndex).toBeGreaterThan(-1);
    expect(imIndex).toBe(usageIndex - 1);
    expect(settingsPages[imIndex]).toMatchObject({
      labelKey: "settings.pages.im",
      searchPlaceholderKey: "settings.search.im",
      badge: 5,
    });
  });

  it("registers Prompt Hooks after Skills", () => {
    const skillsIndex = settingsPages.findIndex((page) => page.id === "skills");
    const promptHooksIndex = settingsPages.findIndex((page) => page.id === "prompt-hooks");

    expect(skillsIndex).toBeGreaterThan(-1);
    expect(promptHooksIndex).toBe(skillsIndex + 1);
    expect(settingsPages[promptHooksIndex]).toMatchObject({
      labelKey: "settings.pages.promptHooks",
      searchPlaceholderKey: "settings.search.promptHooks",
      badge: 7,
    });
  });

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
