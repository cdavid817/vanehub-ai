import { expect, test, type Locator, type Page, type TestInfo } from "@playwright/test";

// Headings match the sidebar entry and breadcrumb for the same page; they used to differ.
const locales = [
  { id: "en", heading: "Basic Configuration" },
  { id: "zh-TW", heading: "基礎配置" },
  { id: "ja", heading: "基本構成" },
  { id: "ko", heading: "기본 구성" },
  { id: "zh-CN", heading: "基础配置" },
] as const;

test.describe.configure({ timeout: 120_000 });

async function openBasicSettings(page: Page) {
  const localeSelect = page.locator("select").filter({ has: page.locator('option[value="zh-TW"]') });
  const settingsButton = page.getByRole("button", { name: /设置|Settings|設定|설정/ });
  const alreadyOpen = await Promise.race([
    localeSelect.waitFor({ state: "visible" }).then(() => true),
    settingsButton.waitFor({ state: "visible" }).then(() => false),
  ]);
  if (!alreadyOpen) await settingsButton.click();
  await expect(localeSelect).toBeVisible();
  return localeSelect;
}

async function expectLayoutFits(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  const selectBox = await page.locator("select").filter({ has: page.locator('option[value="zh-TW"]') }).boundingBox();
  expect(selectBox).not.toBeNull();
  if (selectBox) {
    expect(selectBox.x).toBeGreaterThanOrEqual(0);
    expect(selectBox.x + selectBox.width).toBeLessThanOrEqual(await page.evaluate(() => window.innerWidth));
  }
}

async function expectElementFitsViewport(page: Page, locator: Locator) {
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();
  if (!box) return;
  const viewport = page.viewportSize();
  expect(viewport).not.toBeNull();
  if (!viewport) return;
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.y + box.height).toBeLessThanOrEqual(viewport.height);
}

for (const viewport of [
  { name: "desktop", width: 1440, height: 900 },
  { name: "narrow", width: 390, height: 844 },
]) {
  test(`${viewport.name} switches and restores every application locale without horizontal clipping`, async ({ page }, testInfo: TestInfo) => {
    await page.setViewportSize(viewport);
    await page.goto("/");
    let localeSelect = await openBasicSettings(page);

    for (const locale of locales) {
      await localeSelect.selectOption(locale.id);
      await expect(page.locator("html")).toHaveAttribute("lang", locale.id);
      await expect(page.getByRole("heading", { name: locale.heading, level: 2 })).toBeVisible();
      await expect(localeSelect).toHaveValue(locale.id);
      await expect.poll(async () => page.evaluate(() => JSON.parse(localStorage.getItem("vanehub.appSettings") ?? "{}").applicationLanguage)).toBe(locale.id);
      await expectLayoutFits(page);
      await page.screenshot({ fullPage: true, path: testInfo.outputPath(`${viewport.name}-${locale.id}.png`) });

      await page.reload();
      await expect(page.locator("html")).toHaveAttribute("lang", locale.id);
      localeSelect = await openBasicSettings(page);
      await expect(localeSelect).toHaveValue(locale.id);
    }
  });

  test(`${viewport.name} keeps Japanese workspace, notification, and create-session dialog surfaces inside the viewport`, async ({ page }, testInfo: TestInfo) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "ja" }));
    });
    await page.setViewportSize(viewport);
    await page.goto("/");

    await expect(page.locator("html")).toHaveAttribute("lang", "ja");
    await expect(page.getByText("このタブを使用するには、使用可能なワークスペースのあるセッションを選択します。", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: "通知", exact: true }).click();
    const notificationCenter = page.getByRole("dialog", { name: "通知" });
    await expect(notificationCenter.getByText("皆さんも追い込まれていますね")).toBeVisible();
    await expectElementFitsViewport(page, notificationCenter);
    await page.keyboard.press("Escape");

    await page.getByRole("button", { name: "新しい", exact: true }).click();
    const createHeading = page.getByRole("heading", { name: "セッションの作成" });
    await expect(createHeading).toBeVisible();
    // Task 11.3-11.7's 4-step wizard replaced the old single-screen dialog's static description
    // with a step counter (`createSession.step`); Step 1 reads "ステップ 1 / 4" in Japanese.
    await expect(page.getByText("ステップ 1 / 4")).toBeVisible();
    // The dialog moved onto the shared ApplicationDialog primitive, which is identified by role.
    const createDialog = page.getByRole("dialog", { name: "セッションの作成" });
    await expectElementFitsViewport(page, createDialog);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({ fullPage: true, path: testInfo.outputPath(`${viewport.name}-ja-representative-surfaces.png`) });
    await page.getByRole("button", { name: "キャンセル", exact: true }).click();
  });
}
