import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

test.setTimeout(180_000);

type Theme = "futuristic" | "minimal";

async function prepareSession(page: Page, theme: Theme, width: number) {
  await page.setViewportSize({ width, height: width < 900 ? 844 : 1000 });
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({
      applicationLanguage: "zh-CN",
      theme: selectedTheme,
    }));
  }, theme);
  await page.goto("/", { timeout: 120_000, waitUntil: "domcontentloaded" });
  await createSession(page, `IM ${theme} 会话`);
}

async function configureTelegram(page: Page) {
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /IM 能力|IM Connectors/ }).click();
  await expect(page.locator("[data-connector]")).toHaveCount(5);
  await expect(page.getByText(/Agent 和项目路由由各个会话管理/)).toBeVisible();
  await expect(page.getByRole("button", { name: /保存路由|Save Routing/ })).toHaveCount(0);

  const telegram = page.locator('[data-connector="telegram"]');
  await telegram.getByRole("button", { expanded: false }).click();
  await telegram.getByLabel("Bot Token").fill("playwright-private-token");
  await telegram.getByRole("button", { name: "保存凭据" }).click();
  await expect(page.getByText("连接器凭据已保存。")).toBeVisible();
  await expect(telegram.getByLabel("Bot Token")).toHaveValue("");
  await expect(page.locator("body")).not.toContainText("playwright-private-token");
  const enabled = telegram.locator('input[type="checkbox"]');
  await expect(enabled).toBeEnabled();
  await enabled.check();
  await expect(telegram.getByText("已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "从会话连接" }).click();
}

async function exerciseBinding(page: Page) {
  await page.getByRole("button", { name: "Telegram" }).click();
  await expect(page.getByText(/\/bind [A-Z0-9]{8}/)).toBeVisible();
  await expect(page.getByRole("button", { name: "暂停" })).toBeVisible({ timeout: 10_000 });
  await page.getByRole("checkbox", { name: "完成通知" }).check();
  await page.getByRole("button", { name: "暂停" }).click();
  await expect(page.getByRole("button", { name: "恢复" })).toBeVisible();
  await page.getByRole("button", { name: "移除" }).click();
  await expect(page.getByText(/确定移除当前会话的 IM 连接/)).toBeVisible();
  await page.getByRole("button", { name: "移除连接" }).click();
  await expect(page.getByText("连接当前会话")).toBeVisible();
}

test("desktop session panel pairs and manages IM in futuristic style", async ({ page }) => {
  await prepareSession(page, "futuristic", 1440);
  await configureTelegram(page);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "futuristic");

  await page.getByRole("tab", { name: "IM" }).click();
  await exerciseBinding(page);
});

test("responsive session action opens equivalent IM flow in minimal style", async ({ page }) => {
  await prepareSession(page, "minimal", 390);
  await configureTelegram(page);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");

  const openIm = page.getByRole("button", { name: "打开 IM 连接" });
  await openIm.focus();
  await expect(openIm).toBeFocused();
  await openIm.press("Enter");
  await expect(page.locator('[data-testid="session-im-pane"]')).toBeVisible();
  await exerciseBinding(page);
});
