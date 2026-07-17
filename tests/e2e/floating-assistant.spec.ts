import { expect, test } from "@playwright/test";

test.describe("floating assistant web preview", () => {
  test("opens the quick menu and navigates the mini-chat states", async ({ page }) => {
    await page.goto("/?surface=floating-assistant");

    await page.getByRole("button", { name: /打开悬浮助手菜单/ }).click();
    await expect(page.getByText("VaneHub 助手")).toBeVisible();
    await expect(page.getByRole("button", { name: "新建会话" })).toBeVisible();
    await expect(page.getByRole("button", { name: "返回当前会话" })).toBeVisible();

    await page.getByRole("button", { name: "迷你聊天" }).click();
    await expect(page.getByText("请先在主窗口创建或选择会话。")).toBeVisible();
    await expect(page.getByRole("button", { name: "新建会话" })).toBeVisible();
    await expect(page.getByRole("button", { name: "收起悬浮助手" })).toBeVisible();
    await expect(page.getByText("沿用当前会话配置")).toBeVisible();

    await page.getByRole("button", { name: "收起悬浮助手" }).click();
    await expect(page.getByRole("button", { name: /打开悬浮助手菜单/ })).toBeVisible();

    await page.getByRole("button", { name: /打开悬浮助手菜单/ }).click();
    await page.getByRole("button", { name: "迷你聊天" }).click();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("button", { name: "迷你聊天" })).toBeVisible();
  });

  test("uses the active session configuration and guards a streaming send", async ({ page }) => {
    await page.goto("/?surface=floating-assistant");
    await page.evaluate(async () => {
      const { webAgentClient } = await import("/src/services/web-agent-client.ts");
      await webAgentClient.createSession({
        agentId: "codex-cli",
        interactionMode: "cli",
        title: "悬浮会话",
      });
    });

    await page.getByRole("button", { name: /打开悬浮助手菜单/ }).click();
    await expect(page.getByText("悬浮会话")).toBeVisible();
    await page.getByRole("button", { name: "迷你聊天" }).click();
    await expect(page.getByText(/当前会话就绪/)).toBeVisible();
    await page.getByPlaceholder("向当前会话发送消息…").fill("mini chat message");
    await page.getByRole("button", { name: "发送" }).click();

    await expect(page.getByText("mini chat message")).toBeVisible();
    await expect(page.getByRole("button", { name: "停止" })).toBeVisible();
    await expect(page.getByText(/正在生成回复/)).toBeVisible();
    await expect(page.getByPlaceholder("向当前会话发送消息…")).toBeDisabled();

    await page.getByRole("button", { name: "停止" }).click();
    await expect(page.getByText(/生成已停止/)).toBeVisible();
  });

  for (const variant of [
    { language: "zh-CN", theme: "minimal", menu: "迷你聊天" },
    { language: "en", theme: "futuristic", menu: "Mini chat" },
  ] as const) {
    test(`renders ${variant.theme} in ${variant.language}`, async ({ page }) => {
      await page.addInitScript(({ language, theme }) => {
        localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: language, theme }));
      }, variant);
      await page.goto("/?surface=floating-assistant");
      await page.getByRole("button", { name: /floating assistant menu|悬浮助手菜单/i }).click();

      await expect(page.getByRole("button", { name: variant.menu })).toBeVisible();
      await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe(variant.theme);
    });
  }
});
