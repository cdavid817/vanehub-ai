import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("frontend feature module boundaries", () => {
  it("keeps every settings page behind a first-visit dynamic import", () => {
    // The loaders moved into their own module; the page list next door must still never reach a
    // page through a static import.
    const loaders = read("settings/settings-page-loaders.ts");
    const pages = read("settings/settings-pages.ts");
    // A page may live in its own directory rather than under `pages/`, so the count is of dynamic
    // imports rather than of one path shape; what must not appear is a static import of either.
    const pageModules = loaders.match(/import\("\.\/[^"]+"\)/g) ?? [];
    expect(pageModules).toHaveLength(19);
    expect(loaders).not.toMatch(/from "\.\/pages\//);
    expect(loaders).not.toMatch(/from "\.\/cli-parameters\//);
    expect(pages).not.toMatch(/from "\.\/pages\//);
  });

  it("warms the settings modules used by reload-heavy browser tests", () => {
    const viteConfig = read("../vite.config.ts");
    expect(viteConfig).toContain('"./src/settings/pages/basic-settings-page.tsx"');
    expect(viteConfig).toContain('"./src/settings/pages/agent-configurations-page.tsx"');
  });

  it("keeps Loop Center and non-default session tabs behind dynamic imports", () => {
    const mainLayout = read("main-layout/main-layout.tsx");
    const sessionTabs = read("session-workspace/session-tabs.tsx");
    expect(mainLayout).toContain('import("../loop-center/loop-center")');
    expect(mainLayout).not.toContain('from "../loop-center/loop-center"');
    expect(sessionTabs).toContain('import("./logs-tab")');
    expect(sessionTabs).toContain('import("./report-tab")');
    expect(sessionTabs).not.toContain('from "./logs-tab"');
    expect(sessionTabs).not.toContain('from "./report-tab"');
  });

  it("retains visited settings and tab panels in mounted collections", () => {
    const settingsShell = read("settings/settings-shell.tsx");
    const sessionTabs = read("session-workspace/session-tabs.tsx");
    expect(settingsShell).toContain("new Set([initialPage])");
    expect(settingsShell).toContain("if (!visitedPages.has(page.id)) return null");
    expect(settingsShell).toContain("new Set(current).add(pageId)");
    expect(settingsShell).toContain("hidden={page.id !== activePageId}");
    expect(sessionTabs).toContain("mountedTabs");
    expect(sessionTabs).toContain('activeTab === id ? "block" : "hidden"');
  });

  it("recreates a rejected lazy component when the user retries", () => {
    const boundary = read("components/lazy-feature.tsx");
    expect(boundary).toContain("onReset");
    expect(boundary).toContain("setLazyComponent(lazy(loader))");
    expect(boundary).toContain("featureLoad.retry");
  });

  it("keeps optimized surfaces on the shared service boundary in both runtimes", () => {
    const promptHooks = read("settings/pages/prompt-hooks-page.tsx");
    const logs = read("session-workspace/logs-tab.tsx");
    const webFixtures = read("services/web-session-workspace-fixtures.ts");
    expect(`${promptHooks}\n${logs}`).toContain("agentService");
    expect(`${promptHooks}\n${logs}`).not.toContain("invoke(");
    expect(webFixtures).toContain('fixture: "virtual-scroll"');
  });
});

function read(path: string) {
  return readFileSync(new URL(path, new URL("./", import.meta.url)), "utf8");
}
