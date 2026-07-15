import { expect, test } from "@playwright/test";

test.describe("main chat experience", () => {
  test("shows an empty chat state before a session is selected", async ({ page }) => {
    await page.goto("/");

    await expect(page.getByText("请选择或新建会话")).toBeVisible();
    await expect(page.getByPlaceholder("请选择会话后发送消息")).toBeDisabled();
  });

  test("creates a session, sends a prompt, and renders the mock stream", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /新建/ }).click();
    await expect(page.getByText("开始新的对话")).toBeVisible();

    await page.getByPlaceholder("输入指令，下发任务给当前 Agent...").fill("hello from playwright");
    await page.getByRole("button", { name: "发送" }).click();

    await expect(page.getByText("hello from playwright")).toBeVisible();
    await expect(page.getByText(/Mock .* response|Desktop preview response/)).toBeVisible();
  });

  test("exposes stop while a response is streaming", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /新建/ }).click();
    await page.getByPlaceholder("输入指令，下发任务给当前 Agent...").fill("please stream long enough to stop");
    await page.getByRole("button", { name: "发送" }).click();

    await expect(page.getByRole("button", { name: "停止" })).toBeVisible();
  });

  test("keeps messages scoped to the active session", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /新建/ }).click();
    await page.getByPlaceholder("输入指令，下发任务给当前 Agent...").fill("session one marker");
    await page.getByRole("button", { name: "发送" }).click();
    await expect(page.getByText("session one marker")).toBeVisible();

    await page.getByRole("button", { name: /新建/ }).click();
    await expect(page.getByText("开始新的对话")).toBeVisible();
    await expect(page.getByText("session one marker")).toBeHidden();
  });
});
