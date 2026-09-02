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
    // 20 after merging both branches: 19 from the CLI-parameter cutover plus the Local media page.
    const pageModules = loaders.match(/import\("\.\/[^"]+"\)/g) ?? [];
    expect(pageModules).toHaveLength(20);
    expect(loaders).not.toMatch(/from "\.\/pages\//);
    expect(loaders).not.toMatch(/from "\.\/cli-parameters\//);
    expect(pages).not.toMatch(/from "\.\/pages\//);
  });

  it("warms the settings modules used by reload-heavy browser tests", () => {
    const viteConfig = read("../vite.config.ts");
    expect(viteConfig).toContain('"./src/settings/pages/basic-settings-page.tsx"');
    expect(viteConfig).toContain('"./src/settings/pages/agent-configurations-page.tsx"');
  });

  it("keeps every non-Sessions domain's heavy content and non-default session surfaces behind dynamic imports", () => {
    // Moved out of main-layout.tsx into per-domain destination components (redesign-unified-
    // workbench-ui section 4/5) — main-layout.tsx no longer imports any of these at all now that
    // Runs/Plan/Quality render through RunsDestination/PlanDestination/QualityDestination.
    const runsDestination = read("main-layout/runs-destination.tsx");
    const planDestination = read("main-layout/plan-destination.tsx");
    const qualityDestination = read("main-layout/quality-destination.tsx");
    const mainLayout = read("main-layout/main-layout.tsx");
    // Terminal History/Shell/Logs/Traces moved to the Runtime Panel (redesign-unified-workbench-ui
    // section 8); Changes/Files/Report stayed the primary surfaces' own concern.
    const primarySurfaces = read("session-workspace/session-primary-surfaces.tsx");
    const runtimePanel = read("session-workspace/session-runtime-panel.tsx");
    expect(runsDestination).toContain('import("../loop-center/loop-center")');
    expect(runsDestination).toContain('import("../mission-control/mission-control")');
    expect(runsDestination).toContain('import("../scheduled-tasks/scheduled-tasks-panel")');
    expect(runsDestination).not.toMatch(/from "(\.\.\/loop-center\/loop-center|\.\.\/mission-control\/mission-control|\.\.\/scheduled-tasks\/scheduled-tasks-panel)"/);
    expect(planDestination).toContain('import("../work-board/work-board")');
    expect(planDestination).toContain('import("../goal-center/goal-center")');
    expect(planDestination).not.toMatch(/from "(\.\.\/work-board\/work-board|\.\.\/goal-center\/goal-center)"/);
    expect(qualityDestination).toContain('import("../evaluation-center/evaluation-center")');
    expect(qualityDestination).not.toContain('from "../evaluation-center/evaluation-center"');
    for (const heavyModule of ["loop-center/loop-center", "mission-control/mission-control", "work-board/work-board", "goal-center/goal-center", "evaluation-center/evaluation-center"]) {
      expect(mainLayout).not.toContain(`"../${heavyModule}"`);
    }
    expect(runtimePanel).toContain('import("./logs-tab")');
    expect(runtimePanel).not.toContain('from "./logs-tab"');
    expect(primarySurfaces).toContain('import("./report-tab")');
    expect(primarySurfaces).not.toContain('from "./report-tab"');
  });

  it("retains visited settings and primary session surfaces in mounted collections", () => {
    const settingsShell = read("settings/settings-shell.tsx");
    const primarySurfaces = read("session-workspace/session-primary-surfaces.tsx");
    expect(settingsShell).toContain("new Set([initialPage])");
    expect(settingsShell).toContain("if (!shouldRenderPage(SETTINGS_PAGE_LIFECYCLE[page.id], isActivePage, visitedPages.has(page.id))) return null");
    expect(settingsShell).toContain("new Set(current).add(pageId)");
    expect(settingsShell).toContain("hidden={!isActivePage}");
    expect(primarySurfaces).toContain("mountedTabs");
    expect(primarySurfaces).toContain('scope.activePrimarySurface === id ? "block" : "hidden"');
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
