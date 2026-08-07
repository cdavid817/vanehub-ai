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

function text(locale: Locale, zh: string, en: string) {
  return locale === "en" ? en : zh;
}

/** Opens the create-session dialog with the project and title fields already filled. */
async function openCreateSessionDialog(page: Page, locale: Locale): Promise<Locator> {
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
      await page.goto("/", { waitUntil: "domcontentloaded" });
      await page.addStyleTag({
        content: `
          *, *::before, *::after {
            animation-duration: 0s !important;
            caret-color: transparent !important;
            transition-duration: 0s !important;
          }
          body {
            font-family: Arial, "Microsoft YaHei UI", sans-serif !important;
          }
        `,
      });

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
      // bound remains below 0.3% of this fixed-size dialog screenshot.
      expect(image).toMatchSnapshot(definition.path.split("/"), {
        maxDiffPixels,
      });
    });
  }
});
