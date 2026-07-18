import { describe, expect, it } from "vitest";
import { settingsPages } from "./settings-pages";

describe("settingsPages", () => {
  it("registers Plugin Integrations after Extension Capabilities and before MCP", () => {
    const extensionsIndex = settingsPages.findIndex((page) => page.id === "extensions");
    const pluginsIndex = settingsPages.findIndex((page) => page.id === "plugins");
    const mcpIndex = settingsPages.findIndex((page) => page.id === "mcp");

    expect(pluginsIndex).toBeGreaterThan(-1);
    expect(pluginsIndex).toBe(extensionsIndex + 1);
    expect(pluginsIndex).toBe(mcpIndex - 1);
    expect(settingsPages[pluginsIndex]).toMatchObject({
      labelKey: "settings.pages.plugins",
      searchPlaceholderKey: "settings.search.plugins",
      badge: 1,
    });
  });

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
