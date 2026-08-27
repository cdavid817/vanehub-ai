// @vitest-environment jsdom
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import type { PlaywrightTestConfig } from "@playwright/test";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// playwright.config.ts belongs to the tsconfig.node.json composite project, so a static import
// from src trips TS6305. Loading it through a variable specifier keeps the assertions on the
// real resolved config object instead of degrading them into a scan of the file's text.
const loadPlaywrightConfig = async (): Promise<PlaywrightTestConfig> => {
  const specifier = "../playwright.config";
  return ((await import(specifier)) as { default: PlaywrightTestConfig }).default;
};

// The desktop readiness instrumentation is a runtime branch in main.tsx, not a separate
// entry point, so the only way to prove Web/mock startup stays clear of it is to boot the
// real entry module twice and compare what each boot actually did to the document.
const desktopPlugin = vi.hoisted(() => ({ loads: 0 }));

vi.mock("@wdio/tauri-plugin", () => {
  desktopPlugin.loads += 1;
  return { default: {} };
});

vi.mock("./App", () => ({ App: () => null }));
vi.mock("./floating-assistant/floating-assistant-root", () => ({ FloatingAssistantRoot: () => null }));
// Compiling the Tailwind entry sheet costs ~20s per cold run and contributes nothing to the
// branch under test, so the stylesheet import is stubbed rather than processed.
vi.mock("./styles.css", () => ({}));

// jsdom gives this module an http import.meta.url, so the repository root has to come from
// the runner's working directory rather than a file URL.
const projectRoot = process.cwd();

type StartedSurface = {
  root: HTMLElement;
  globalEventTypes: string[];
};

// main.tsx never detaches its window listeners, and jsdom shares one window across the whole
// file, so every boot records what it attached and beforeEach detaches it again. Without this
// a later test would inherit the previous boot's instrumentation and pass for the wrong reason.
type InstalledListener = Parameters<typeof window.addEventListener>;
const installedListeners: InstalledListener[] = [];

async function startSurface(): Promise<StartedSurface> {
  const root = document.createElement("div");
  root.id = "root";
  document.body.append(root);
  const addEventListener = vi.spyOn(window, "addEventListener");
  await import("./main");
  await vi.waitFor(() => {
    expect(root.dataset.vanehubBootstrap).toBe("ready");
  });
  const calls = [...addEventListener.mock.calls];
  addEventListener.mockRestore();
  installedListeners.push(...calls);
  return { root, globalEventTypes: calls.map(([type]) => String(type)) };
}

beforeEach(() => {
  for (const [type, listener, options] of installedListeners.splice(0)) {
    window.removeEventListener(type, listener, options);
  }
  vi.resetModules();
  desktopPlugin.loads = 0;
  document.body.replaceChildren();
});

afterEach(() => {
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

describe("desktop readiness instrumentation boundary", () => {
  it("does not depend on an animation frame to publish React readiness", async () => {
    vi.stubEnv("VITE_DESKTOP_E2E", undefined);
    const requestAnimationFrame = vi.spyOn(window, "requestAnimationFrame")
      .mockImplementation(() => 1);

    const { root } = await startSurface();

    expect(root.dataset.vanehubBootstrap).toBe("ready");
    expect(requestAnimationFrame).not.toHaveBeenCalled();
  });

  it("keeps Web/mock startup free of the desktop automation branch", async () => {
    vi.stubEnv("VITE_DESKTOP_E2E", undefined);

    const { root, globalEventTypes } = await startSurface();

    expect(desktopPlugin.loads).toBe(0);
    expect(globalEventTypes).not.toContain("error");
    expect(globalEventTypes).not.toContain("unhandledrejection");

    window.dispatchEvent(new Event("error"));
    window.dispatchEvent(new Event("unhandledrejection"));

    expect(root.dataset.vanehubFatalError).toBeUndefined();
    expect(root.dataset.vanehubBootstrap).toBe("ready");
  });

  it("loads the desktop automation branch only for an explicit desktop test build", async () => {
    vi.stubEnv("VITE_DESKTOP_E2E", "1");

    const { root, globalEventTypes } = await startSurface();

    expect(desktopPlugin.loads).toBe(1);
    expect(globalEventTypes).toContain("error");
    expect(globalEventTypes).toContain("unhandledrejection");

    window.dispatchEvent(new ErrorEvent("error", { error: new Error("token=desktop-test-secret") }));

    expect(root.dataset.vanehubFatalError).toBe("error");
    expect(root.dataset.vanehubFatalErrorDetail).toBe("token=[REDACTED]");
  });

  it("exposes the same browser-observable startup contract in both runtimes", async () => {
    vi.stubEnv("VITE_DESKTOP_E2E", undefined);
    const web = await startSurface();

    vi.resetModules();
    document.body.replaceChildren();
    vi.stubEnv("VITE_DESKTOP_E2E", "1");
    const desktop = await startSurface();

    expect(web.root.dataset.vanehubBootstrap).toBe(desktop.root.dataset.vanehubBootstrap);
    expect(Object.keys(web.root.dataset)).toEqual(["vanehubBootstrap"]);
    expect(Object.keys(desktop.root.dataset)).toContain("vanehubBootstrap");
  });
});

describe("Playwright browser suite independence", () => {
  it("runs only the browser specs against an un-instrumented dev server", async () => {
    const playwrightConfig = await loadPlaywrightConfig();
    const webServer = playwrightConfig.webServer;
    expect(Array.isArray(webServer)).toBe(false);
    const command = (webServer as { command: string }).command;

    expect(playwrightConfig.testDir).toBe("./tests/e2e");
    expect(command).toContain("npm run dev");
    expect(command).not.toMatch(/VITE_DESKTOP_E2E|desktop-e2e|wdio/);

    const packageJson = readFileSync(path.join(projectRoot, "package.json"), "utf8");
    const devScript = (JSON.parse(packageJson) as { scripts: Record<string, string> }).scripts.dev;
    expect(devScript).not.toMatch(/VITE_DESKTOP_E2E|desktop-e2e|wdio/);
  });

  it("keeps browser specs free of desktop automation dependencies", () => {
    const specDir = path.join(projectRoot, "tests", "e2e");
    const specs = readdirSync(specDir).filter((entry) => entry.endsWith(".ts"));

    expect(specs.length).toBeGreaterThan(0);
    for (const spec of specs) {
      const source = readFileSync(path.join(specDir, spec), "utf8");
      expect(source, spec).not.toMatch(/@wdio|VITE_DESKTOP_E2E|vanehub-fatal-error|vanehubFatalError/);
    }
  });
});
