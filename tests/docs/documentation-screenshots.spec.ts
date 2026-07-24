import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { dirname, resolve } from "node:path";
import { expect, test } from "@playwright/test";

interface ScreenshotDefinition {
  id: string;
  locale: "en" | "zh-CN";
  runtime: "web-mock" | "desktop-reviewed";
  featureState: "delivered" | "preview" | "planned";
  path: string;
}

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const inventory = JSON.parse(
  readFileSync(resolve(repositoryRoot, "docs", "user-guide", "screenshots.json"), "utf8"),
) as { screenshots: ScreenshotDefinition[] };
const mode = process.env.DOCS_SCREENSHOT_MODE;

if (mode !== "update" && mode !== "check") {
  throw new Error("DOCS_SCREENSHOT_MODE must be update or check.");
}

function digest(value: Buffer) {
  return createHash("sha256").update(value).digest("hex");
}

test.describe("documentation screenshots", () => {
  test.describe.configure({ mode: "serial" });

  for (const definition of inventory.screenshots) {
    test(definition.id, async ({ page }, testInfo) => {
      expect(definition.runtime).toBe("web-mock");
      expect(definition.featureState).toBe("delivered");

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

      await page.getByRole("button", { name: /^(新建|New)$/ }).click();
      const dialog = page.locator(".fixed.inset-0").locator(".ucd-panel");
      await expect(
        dialog.getByRole("heading", {
          name: definition.locale === "en" ? "Create Session" : "创建会话",
        }),
      ).toBeVisible();
      await dialog.locator('input[placeholder*="code"]').fill("D:\\VaneHub-Demo");
      await dialog.getByPlaceholder(
        definition.locale === "en" ? "New session" : "新会话",
      ).fill(definition.locale === "en" ? "Documentation demo" : "文档演示");
      await expect(
        dialog.getByRole("button", {
          name: definition.locale === "en" ? "Create" : "创建",
          exact: true,
        }),
      ).toBeEnabled();
      await expect(dialog.getByRole("button", { name: /^Claude Code/ })).toBeVisible();
      await expect(dialog.getByRole("button", { name: /^Gemini CLI/ })).toBeVisible();
      await expect(dialog.getByRole("button", { name: /^Codex CLI/ })).toBeVisible();
      await expect(dialog.getByRole("button", { name: /^OpenCode/ })).toBeVisible();
      const bounds = await dialog.boundingBox();
      expect(bounds?.width).toBeGreaterThanOrEqual(580);
      expect(bounds?.height).toBeGreaterThanOrEqual(650);

      const image = await dialog.screenshot({
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

      if (!existsSync(assetPath)) {
        throw new Error(`${definition.id}: expected documentation asset is missing at ${assetPath}`);
      }
      const expected = readFileSync(assetPath);
      if (!image.equals(expected)) {
        await testInfo.attach(`${definition.id}-actual`, {
          body: image,
          contentType: "image/png",
        });
        throw new Error(
          `${definition.id}: screenshot is stale ` +
            `(expected sha256 ${digest(expected)}, received ${digest(image)}). ` +
            "Review the UI and run npm run docs:screenshots:update intentionally.",
        );
      }
    });
  }
});
