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
    });
    expect(settingsPages[imIndex].badge).toBeUndefined();
  });

  it("registers Plugin Integrations after Extension Capabilities", () => {
    const extensionsIndex = settingsPages.findIndex((page) => page.id === "extensions");
    const pluginsIndex = settingsPages.findIndex((page) => page.id === "plugins");

    expect(pluginsIndex).toBeGreaterThan(-1);
    expect(pluginsIndex).toBe(extensionsIndex + 1);
    expect(settingsPages[pluginsIndex]).toMatchObject({
      labelKey: "settings.pages.plugins",
      searchPlaceholderKey: "settings.search.plugins",
    });
    expect(settingsPages[pluginsIndex].badge).toBeUndefined();
  });

  it("registers Prompt Hooks after Skills", () => {
    const skillsIndex = settingsPages.findIndex((page) => page.id === "skills");
    const promptHooksIndex = settingsPages.findIndex((page) => page.id === "prompt-hooks");

    expect(skillsIndex).toBeGreaterThan(-1);
    expect(promptHooksIndex).toBe(skillsIndex + 1);
    expect(settingsPages[promptHooksIndex]).toMatchObject({
      labelKey: "settings.pages.promptHooks",
      searchPlaceholderKey: "settings.search.promptHooks",
    });
    expect(settingsPages[promptHooksIndex].badge).toBeUndefined();
  });

  it("registers Usage Statistics before About", () => {
    const extensionsIndex = settingsPages.findIndex((page) => page.id === "extensions");
    const pluginsIndex = settingsPages.findIndex((page) => page.id === "plugins");
    const usageIndex = settingsPages.findIndex((page) => page.id === "usage");
    const aboutIndex = settingsPages.findIndex((page) => page.id === "about");

    expect(usageIndex).toBeGreaterThan(-1);
    expect(extensionsIndex).toBeLessThan(usageIndex);
    expect(pluginsIndex).toBe(usageIndex - 1);
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
