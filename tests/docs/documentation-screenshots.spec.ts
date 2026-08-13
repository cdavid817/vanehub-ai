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

/**
 * Fills the create-session dialog without navigating.
 *
 * Web/mock state such as a saved OnePiece provider lives in module memory, so any
 * scenario that configures something first must stay on the same document.
 */
async function fillCreateSessionDialog(page: Page, locale: Locale): Promise<Locator> {
  await page.getByRole("button", { name: /^(新建|New)$/ }).click();
  const dialog = page.locator(".fixed.inset-0").locator(".ucd-panel");
  await expect(
    dialog.getByRole("heading", { name: text(locale, "创建会话", "Create Session") }),
  ).toBeVisible();
  await expect(
    dialog.getByText(text(locale, "会话类型", "Session Type"), { exact: true }),
  ).toBeVisible();
  await dialog.locator('input[placeholder*="code"]').fill("D:\\VaneHub-Demo");
  await dialog
    .getByPlaceholder(text(locale, "新会话", "New session"))
    .fill(text(locale, "文档演示", "Documentation demo"));
  await dialog
    .locator(":scope > .min-h-0.overflow-y-auto")
    .evaluate((scrollRegion) => scrollRegion.scrollTo({ top: 0 }));
  return dialog;
}

/** Loads the workspace and opens the create-session dialog. */
async function openCreateSessionDialog(page: Page, locale: Locale): Promise<Locator> {
  await visit(page, "/");
  return fillCreateSessionDialog(page, locale);
}

/**
 * Creates a session through the dialog and returns the workspace screen.
 *
 * The tab bar is already mounted behind the modal, so waiting for the dialog to
 * close is what separates a real workspace capture from one of the dialog itself.
 */
async function createSession(
  page: Page,
  locale: Locale,
  options: { agent?: string; navigate?: boolean } = {},
): Promise<Locator> {
  const dialog = options.navigate === false
    ? await fillCreateSessionDialog(page, locale)
    : await openCreateSessionDialog(page, locale);
  if (options.agent) {
    await dialog.getByRole("button", { name: new RegExp(`^${options.agent}`) }).first().click();
  }
  await dialog
    .getByRole("button", { name: text(locale, "创建", "Create"), exact: true })
    .click();
  await expect(page.locator(".fixed.inset-0").locator(".ucd-panel")).toHaveCount(0, {
    timeout: 15_000,
  });
  const shell = page.locator("main").first();
  await expect(
    shell.getByRole("tablist", { name: text(locale, "会话工作区", "Session workspace") }),
  ).toBeVisible();
  await dismissToasts(page, locale);
  return shell;
}

/**
 * Toasts auto-dismiss on a timer, so a slower run would capture one half-faded or
 * already gone. Closing them pins the frame.
 */
async function dismissToasts(page: Page, locale: Locale) {
  const toast = page.getByRole("button", {
    name: text(locale, "关闭通知", "Dismiss notification"),
  });
  for (const button of await toast.all()) await button.click();
  await expect(toast).toHaveCount(0, { timeout: 5_000 });
}

/** Switches the open session workspace to a tab and waits for its lazily loaded panel. */
async function openSessionTab(page: Page, label: string) {
  const shell = page.locator("main").first();
  await shell.getByRole("tab", { name: label }).click();
  await expect(shell.getByRole("tab", { name: label })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await waitForFeature(shell);
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

  /** A globally configured connector reaching "connected" before session binding. */
  "im-connected": async (page, locale) => {
    await createSession(page, locale);
    await page.getByRole("button", { name: text(locale, "设置", "Settings") }).click();
    const shell = page.locator("main").first();
    await shell.getByRole("button", { name: text(locale, "IM 能力", "Instant messaging") }).click();
    await waitForFeature(shell);

    // Every settings page stays mounted and only the active one is visible, so every
    // control here must be filtered to what is actually on screen.
    const visible = { visible: true } as const;
    await shell.getByRole("button", { name: /飞书/ }).filter(visible).first().click();
    const credentials = shell
      .locator('input[type="text"], input[type="password"]')
      .filter(visible);
    for (const field of await credentials.all()) await field.fill("web-mock-value");
    await shell
      .getByRole("button", { name: text(locale, "保存凭据", "Save credentials") })
      .filter(visible)
      .click();
    await shell.locator('input[type="checkbox"]').filter(visible).first().check();
    await expect(
      shell.getByText(text(locale, "已连接", "Connected")).filter(visible).first(),
    ).toBeVisible({ timeout: 15_000 });
    await dismissToasts(page, locale);
    const activeSettingsPage = shell.locator("div.h-full.overflow-y-auto").filter(visible);
    await expect(activeSettingsPage).toHaveCount(1);
    await activeSettingsPage.evaluate((element) => {
      element.scrollTop = 0;
      element.scrollLeft = 0;
    });
    await expect.poll(() => activeSettingsPage.evaluate((element) => element.scrollTop)).toBe(0);
    return shell;
  },

  "session-workspace": async (page, locale) => {
    const shell = await createSession(page, locale);
    return shell;
  },

  "session-traces": async (page, locale) => {
    const shell = await createSession(page, locale);
    await openSessionTab(page, text(locale, "链路", "Traces"));
    return shell;
  },

  "session-logs": async (page, locale) => {
    const shell = await createSession(page, locale);
    await openSessionTab(page, text(locale, "日志", "Logs"));
    return shell;
  },

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

/**
 * Elements painted over before capture. The spec requires repeated generation not to
 * introduce unrelated timestamp, id, or path changes, and a live clock in the frame
 * would otherwise make a screenshot impossible to regression-check.
 */
const masks: Record<string, (page: Page) => Locator[]> = {
  "im-connected": (page) => [page.getByText(/更新时间|Updated/)],
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
        mask: masks[definition.scenario]?.(page) ?? [],
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
