import { expect, test, type Page } from "@playwright/test";

type Locale = "en" | "zh-CN";
type Theme = "futuristic" | "minimal";

async function openImSettings(page: Page, language: Locale, theme: Theme) {
  await page.addInitScript(({ applicationLanguage, selectedTheme }) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({
      applicationLanguage,
      theme: selectedTheme,
    }));
  }, { applicationLanguage: language, selectedTheme: theme });
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /IM 能力|IM Connectors/ }).click();
  await expect(page.locator("[data-connector]")).toHaveCount(5);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  await expect(page.locator("body")).not.toContainText(/sentinel-private-value|write-only-token/);
}

for (const scenario of [
  { language: "en", theme: "futuristic", width: 1440, height: 1000 },
  { language: "en", theme: "minimal", width: 390, height: 844 },
  { language: "zh-CN", theme: "futuristic", width: 1440, height: 1000 },
  { language: "zh-CN", theme: "minimal", width: 390, height: 844 },
] as const) {
  test(`${scenario.language} ${scenario.theme} at ${scenario.width}px`, async ({ page }) => {
    await page.setViewportSize({ width: scenario.width, height: scenario.height });
    await openImSettings(page, scenario.language, scenario.theme);

    await expect(page.getByText(scenario.language === "en" ? "Browser preview simulates connector actions. It cannot receive live platform messages or store secrets securely." : "浏览器预览仅模拟连接器操作，不能接收平台实时消息，也不会安全保存凭据。")).toBeVisible();
    await page.getByRole("button", { name: scenario.language === "en" ? "Save Routing" : "保存路由" }).click();
    await expect(page.getByText(scenario.language === "en" ? "Select an available Agent that supports CLI interaction." : "请选择支持 CLI 交互的可用 Agent。")).toBeVisible();

    const viewport = page.locator("main");
    const box = await viewport.boundingBox();
    expect(box?.width).toBeLessThanOrEqual(scenario.width);
    await page.screenshot({
      fullPage: true,
      path: `test-results/im-${scenario.language}-${scenario.theme}-${scenario.width}.png`,
    });
  });
}

test("Web mock supports routing, write-only credentials, and QR transitions", async ({ page }) => {
  await openImSettings(page, "en", "futuristic");
  await page.getByLabel("Default Agent").selectOption("codex-cli");
  await page.getByRole("button", { name: "Browse for project directory" }).click();
  await page.getByRole("button", { name: "Save Routing" }).click();
  await expect(page.getByText("Default routing saved.")).toBeVisible();

  const telegram = page.locator('[data-connector="telegram"]');
  await telegram.getByRole("button", { expanded: false }).click();
  await telegram.getByLabel("Bot Token").fill("playwright-private-token");
  await telegram.getByRole("button", { name: "Save Credentials" }).click();
  await expect(telegram.getByText("Save Credentials in progress...")).toBeVisible();
  await expect(page.locator('[data-connector="feishu"]')).not.toContainText("in progress");
  await expect(page.getByText("Connector credentials saved.")).toBeVisible();
  await expect(telegram.getByLabel("Bot Token")).toHaveValue("");
  await expect(telegram.getByLabel("Bot Token")).toHaveAttribute("placeholder", "Configured - enter a value only to replace");
  await expect(page.locator("body")).not.toContainText("playwright-private-token");

  const wechat = page.locator('[data-connector="weixin"]');
  await wechat.getByRole("button", { expanded: false }).click();
  await wechat.getByRole("button", { name: "Start Authorization" }).click();
  await expect(wechat.getByRole("img", { name: "Short-lived personal WeChat authorization QR code" })).toBeVisible();
  await wechat.getByRole("button", { name: "Check Status" }).click();
  await expect(wechat.getByText("Scanned - confirm on your phone")).toBeVisible();
  await wechat.getByRole("button", { name: "Check Status" }).click();
  await expect(wechat.getByText("Authorization completed")).toBeVisible();
});
