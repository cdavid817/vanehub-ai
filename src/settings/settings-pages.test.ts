import { describe, expect, it } from "vitest";
import { settingsPages } from "./settings-pages";

describe("settingsPages", () => {
  it("orders destinations by the primary settings workflow", () => {
    expect(settingsPages.map((page) => page.id)).toEqual([
      "basic",
      "agent-configurations",
      "agent-policies",
      "cli-parameters",
      "code-intelligence",
      "mcp",
      "skills",
      "personalization",
      "prompt-hooks",
      "expert-roles",
      "local-media",
      "providers",
      "extensions",
      "plugins",
      "im",
      "ssh-connections",
      "observability",
      "usage",
      "help",
      "about",
    ]);
  });
  it("registers every page as a lazy first-visit module", () => {
    expect(settingsPages).toHaveLength(20);
    expect(settingsPages.every((page) => typeof page.loader === "function")).toBe(true);
    expect(settingsPages.every((page) => !("component" in page))).toBe(true);
  });

  it("hides SDK dependencies from primary navigation", () => {
    expect(settingsPages.map((page) => page.id as string)).not.toContain("sdk");
  });

  it("registers Agent configurations as the only Agent settings destination", () => {
    const configurationsIndex = settingsPages.findIndex((page) => page.id === "agent-configurations");

    expect(settingsPages.map((page) => page.id as string)).not.toContain("agents");
    expect(configurationsIndex).toBeGreaterThan(-1);
    expect(settingsPages[configurationsIndex]).toMatchObject({
      labelKey: "settings.pages.agentConfigurations",
      searchPlaceholderKey: "settings.search.agentConfigurations",
    });
    expect(settingsPages[configurationsIndex].loader).toBeTypeOf("function");
  });

  it("registers workspace-wide code intelligence outside Agent configurations", () => {
    const page = settingsPages.find((candidate) => candidate.id === "code-intelligence");
    expect(page).toMatchObject({
      group: "capabilities",
      labelKey: "settings.pages.codeIntelligence",
      searchPlaceholderKey: "settings.search.codeIntelligence",
    });
    expect(page?.loader).toBeTypeOf("function");
  });

  it("registers Personalization after Skills and before Prompt Hooks", () => {
    const configurationsIndex = settingsPages.findIndex((page) => page.id === "agent-configurations");
    const personalizationIndex = settingsPages.findIndex((page) => page.id === "personalization");
    const skillsIndex = settingsPages.findIndex((page) => page.id === "skills");

    expect(skillsIndex).toBeGreaterThan(configurationsIndex);
    expect(personalizationIndex).toBe(skillsIndex + 1);
    expect(settingsPages[personalizationIndex]).toMatchObject({
      labelKey: "settings.pages.personalization",
      searchPlaceholderKey: "settings.search.personalization",
    });
  });

  it("registers extensions below higher-frequency management pages", () => {
    const imIndex = settingsPages.findIndex((page) => page.id === "im");
    const sshIndex = settingsPages.findIndex(
      (page) => page.id === "ssh-connections",
    );
    const extensionsIndex = settingsPages.findIndex(
      (page) => page.id === "extensions",
    );

    expect(imIndex).toBeGreaterThan(-1);
    expect(sshIndex).toBe(imIndex + 1);
    expect(extensionsIndex).toBeLessThan(imIndex);
    expect(settingsPages[imIndex]).toMatchObject({
      labelKey: "settings.pages.im",
      searchPlaceholderKey: "settings.search.im",
    });
    expect(settingsPages[sshIndex]).toMatchObject({
      labelKey: "settings.pages.sshConnections",
      searchPlaceholderKey: "settings.search.sshConnections",
    });
    expect(settingsPages[imIndex].badge).toBeUndefined();
  });

  it("registers Plugin Integrations after Extension Capabilities", () => {
    const extensionsIndex = settingsPages.findIndex(
      (page) => page.id === "extensions",
    );
    const pluginsIndex = settingsPages.findIndex(
      (page) => page.id === "plugins",
    );

    expect(pluginsIndex).toBeGreaterThan(-1);
    expect(pluginsIndex).toBe(extensionsIndex + 1);
    expect(settingsPages[pluginsIndex]).toMatchObject({
      labelKey: "settings.pages.plugins",
      searchPlaceholderKey: "settings.search.plugins",
    });
    expect(settingsPages[pluginsIndex].badge).toBeUndefined();
  });

  it("registers Prompt Hooks after Personalization", () => {
    const personalizationIndex = settingsPages.findIndex((page) => page.id === "personalization");
    const promptHooksIndex = settingsPages.findIndex(
      (page) => page.id === "prompt-hooks",
    );

    expect(personalizationIndex).toBeGreaterThan(-1);
    expect(promptHooksIndex).toBe(personalizationIndex + 1);
    expect(settingsPages[promptHooksIndex]).toMatchObject({
      labelKey: "settings.pages.promptHooks",
      searchPlaceholderKey: "settings.search.promptHooks",
    });
    expect(settingsPages[promptHooksIndex].badge).toBeUndefined();
  });

  it("registers Usage Statistics before About", () => {
    const extensionsIndex = settingsPages.findIndex(
      (page) => page.id === "extensions",
    );
    const pluginsIndex = settingsPages.findIndex(
      (page) => page.id === "plugins",
    );
    const usageIndex = settingsPages.findIndex((page) => page.id === "usage");
    const aboutIndex = settingsPages.findIndex((page) => page.id === "about");

    expect(usageIndex).toBeGreaterThan(-1);
    expect(extensionsIndex).toBeLessThan(usageIndex);
    const observabilityIndex = settingsPages.findIndex(
      (page) => page.id === "observability",
    );
    expect(observabilityIndex).toBe(usageIndex - 1);
    expect(pluginsIndex).toBeLessThan(observabilityIndex);
    const helpIndex = settingsPages.findIndex((page) => page.id === "help");
    expect(usageIndex).toBe(helpIndex - 1);
    expect(helpIndex).toBe(aboutIndex - 1);
    expect(settingsPages[usageIndex]).toMatchObject({
      labelKey: "settings.pages.usage",
      searchPlaceholderKey: "settings.search.usage",
    });
  });

  it("registers execution observability before usage statistics", () => {
    const observabilityIndex = settingsPages.findIndex(
      (page) => page.id === "observability",
    );
    expect(settingsPages[observabilityIndex]).toMatchObject({
      labelKey: "settings.pages.observability",
      searchPlaceholderKey: "settings.search.observability",
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
