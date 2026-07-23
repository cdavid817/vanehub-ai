import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("frontend feature module boundaries", () => {
  it("keeps heavy settings pages behind dynamic imports", () => {
    const source = read("settings/settings-pages.ts");
    expect(source).toContain('import("./pages/agents-page")');
    expect(source).toContain('import("./pages/prompt-hooks-page")');
    expect(source).not.toContain('from "./pages/agents-page"');
    expect(source).not.toContain('from "./pages/prompt-hooks-page"');
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
    expect(settingsShell).toContain("visitedLazyPages");
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
    const webFixtures = read("services/web-session-workspace-client.ts");
    expect(`${promptHooks}\n${logs}`).toContain("agentService");
    expect(`${promptHooks}\n${logs}`).not.toContain("invoke(");
    expect(webFixtures).toContain('fixture: "virtual-scroll"');
  });
});

function read(path: string) {
  return readFileSync(new URL(path, new URL("./", import.meta.url)), "utf8");
}
