import {
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { dirname, resolve } from "node:path";
import { expect, test, type Locator, type Page } from "@playwright/test";

type Locale = "en" | "zh-CN";

interface ScreenshotDefinition {
  id: string;
  scenario: string;
  locale: Locale;
  runtime: "web-mock" | "desktop-reviewed";
  featureState: "delivered" | "preview" | "planned";
  path: string;
}

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const inventory = JSON.parse(
  readFileSync(resolve(repositoryRoot, "docs", "user-guide", "screenshots.json"), "utf8"),
) as { screenshots: ScreenshotDefinition[] };
const mode = process.env.DOCS_SCREENSHOT_MODE;
const maxDiffPixels = 1_000;

if (mode !== "update" && mode !== "check") {
  throw new Error("DOCS_SCREENSHOT_MODE must be update or check.");
}

/** Removes animation, caret, and font variance so repeated runs rasterize identically. */
const deterministicCss = `
  *, *::before, *::after {
    animation-duration: 0s !important;
    caret-color: transparent !important;
    transition-duration: 0s !important;
  }
  body {
    font-family: Arial, "Microsoft YaHei UI", sans-serif !important;
  }
`;

function text(locale: Locale, zh: string, en: string) {
  return locale === "en" ? en : zh;
}

async function visit(page: Page, path: string) {
  await page.goto(path, { waitUntil: "domcontentloaded" });
  await page.addStyleTag({ content: deterministicCss });
}

/**
 * Feature surfaces are lazily loaded, so the shell renders long before its content.
 * Without this wait a capture silently freezes on "Loading feature..." — the surrounding
 * chrome is visible either way, so asserting on a label alone proves nothing.
 */
async function waitForFeature(shell: Locator) {
  await expect(shell.getByText(/正在加载功能|Loading feature/)).toHaveCount(0, {
    timeout: 15_000,
  });
}

/** Opens a settings section by route and returns the settings screen. */
async function openSettings(page: Page, section: string, heading: string): Promise<Locator> {
  await visit(page, `/settings?section=${section}`);
  const shell = page.locator("main").first();
  await expect(shell).toBeVisible();
  await waitForFeature(shell);
  // The top bar repeats the section name as an h1, so match the content heading by level.
  await expect(shell.getByRole("heading", { level: 2, name: heading })).toBeVisible();
  return shell;
}

/** Opens the create-session dialog with the project and title fields already filled. */
async function openCreateSessionDialog(page: Page, locale: Locale): Promise<Locator> {
  await visit(page, "/");
  await page.getByRole("button", { name: /^(新建|New)$/ }).click();
  const dialog = page.locator(".fixed.inset-0").locator(".ucd-panel");
  await expect(
    dialog.getByRole("heading", { name: text(locale, "创建会话", "Create Session") }),
  ).toBeVisible();
  await dialog.locator('input[placeholder*="code"]').fill("D:\\VaneHub-Demo");
  await dialog
    .getByPlaceholder(text(locale, "新会话", "New session"))
    .fill(text(locale, "文档演示", "Documentation demo"));
  return dialog;
}

/**
 * Each scenario returns the element to capture. Scenarios must only assert on
 * user-visible controls: a Web/mock capture is never evidence of a native side effect.
 */
const scenarios: Record<string, (page: Page, locale: Locale) => Promise<Locator>> = {
  "create-session": async (page, locale) => {
    const dialog = await openCreateSessionDialog(page, locale);
    await expect(
      dialog.getByRole("button", { name: text(locale, "创建", "Create"), exact: true }),
    ).toBeEnabled();
    await expect(dialog.getByRole("button", { name: /^Claude Code/ })).toBeVisible();
    await expect(dialog.getByRole("button", { name: /^Gemini CLI/ })).toBeVisible();
    await expect(dialog.getByRole("button", { name: /^Codex CLI/ })).toBeVisible();
    await expect(dialog.getByRole("button", { name: /^OpenCode/ })).toBeVisible();
    const bounds = await dialog.boundingBox();
    expect(bounds?.width).toBeGreaterThanOrEqual(580);
    expect(bounds?.height).toBeGreaterThanOrEqual(650);
    return dialog;
  },

  /**
   * Multi-Agent seat assignment. This scenario exists because the guide previously
   * documented Multi Agent as disabled, which is no longer true.
   */
  "create-session-multi-agent": async (page, locale) => {
    const dialog = await openCreateSessionDialog(page, locale);
    const multiAgent = dialog.getByRole("button", {
      name: new RegExp(`^${text(locale, "多 Agent", "Multi Agent")}`),
    });
    await expect(multiAgent).toBeEnabled();
    await multiAgent.click();
    await expect(multiAgent).toHaveAttribute("aria-pressed", "true");
    const bounds = await dialog.boundingBox();
    expect(bounds?.width).toBeGreaterThanOrEqual(580);
    return dialog;
  },

  "settings-agent-policies": (page, locale) =>
    openSettings(page, "agent-policies", text(locale, "Agent 权限策略", "Agent policies")),

  "settings-personalization": (page, locale) =>
    openSettings(page, "personalization", text(locale, "个性化", "Personalization")),

  "settings-expert-roles": (page, locale) =>
    openSettings(page, "expert-roles", text(locale, "专家角色", "Expert roles")),

  "settings-mcp": (page, locale) =>
    openSettings(page, "mcp", text(locale, "MCP 服务器", "MCP servers")),

  "settings-cli": (page, locale) =>
    openSettings(page, "providers", text(locale, "CLI 管理", "CLI management")),

  "settings-usage": (page, locale) =>
    openSettings(page, "usage", text(locale, "使用统计", "Usage")),

  "settings-skills": (page, locale) =>
    openSettings(page, "skills", text(locale, "Skill 管理", "Skills")),

  "settings-prompt-hooks": (page, locale) =>
    openSettings(page, "prompt-hooks", text(locale, "Prompt Hook", "Prompt Hooks")),

  "settings-im": (page, locale) =>
    openSettings(page, "im", text(locale, "IM 能力", "Instant messaging")),

  "settings-ssh": (page, locale) =>
    openSettings(page, "ssh-connections", text(locale, "SSH 连接", "SSH connections")),

  "settings-extensions": (page, locale) =>
    openSettings(page, "extensions", text(locale, "扩展能力", "Extensions")),

  "settings-observability": (page, locale) =>
    openSettings(page, "observability", text(locale, "执行可观测性", "Execution observability")),

  "scheduled-tasks": async (page, locale) => {
    await visit(page, "/");
    await page
      .getByRole("button", { name: text(locale, "定时任务", "Scheduled tasks"), exact: true })
      .first()
      .click();
    const dialog = page.locator(".fixed.inset-0").locator(".ucd-panel");
    await expect(
      dialog.getByRole("heading", { name: text(locale, "定时任务", "Scheduled tasks") }),
    ).toBeVisible();
    return dialog;
  },

  "loop-center": async (page, locale) => {
    await visit(page, "/");
    await page
      .getByRole("button", { name: text(locale, "循环工程", "Loops"), exact: true })
      .click();
    const shell = page.locator("main").first();
    await expect(shell).toBeVisible();
    await waitForFeature(shell);
    return shell;
  },
};

test.describe("documentation screenshots", () => {
  test.describe.configure({ mode: "serial" });

  for (const definition of inventory.screenshots) {
    test(definition.id, async ({ page }) => {
      expect(definition.runtime).toBe("web-mock");
      expect(definition.featureState).toBe("delivered");
      const capture = scenarios[definition.scenario];
      expect(capture, `unknown scenario "${definition.scenario}"`).toBeTruthy();

      await page.setViewportSize({ width: 1440, height: 900 });
      await page.addInitScript(({ locale }) => {
        localStorage.clear();
        localStorage.setItem(
          "vanehub.appSettings",
          JSON.stringify({
            applicationLanguage: locale,
            fontSize: "medium",
            theme: "minimal",
          }),
        );
      }, { locale: definition.locale });

      const target = await capture(page, definition.locale);
      const image = await target.screenshot({
        animations: "disabled",
        caret: "hide",
        scale: "css",
      });
      const assetPath = resolve(repositoryRoot, "docs", "user-guide", definition.path);

      if (mode === "update") {
        mkdirSync(dirname(assetPath), { recursive: true });
        writeFileSync(assetPath, image);
        return;
      }

      // Hosted Windows Chromium can rasterize one-pixel borders across several form
      // controls on adjacent rows even when layout and content are unchanged. The
      // bound remains below 0.3% of a fixed-size capture.
      expect(image).toMatchSnapshot(definition.path.split("/"), {
        maxDiffPixels,
      });
    });
  }
});
