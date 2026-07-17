import { expect, test, type Page } from "@playwright/test";

async function createSession(page: Page, title: string) {
  await page.getByRole("button", { name: /新建|New/ }).click();
  await expect(page.getByRole("heading", { name: /创建会话|Create Session/ })).toBeVisible();
  await page.locator('input[placeholder^="D:"]').fill("D:\\example-workspace");
  await page.getByPlaceholder(/新会话|New session/).fill(title);
  await page.getByRole("button", { name: /^创建$|^Create$/ }).click();
  await expect(page.getByText(/开始新的对话|Start a new conversation/)).toBeVisible();
}

test.describe("main chat experience", () => {
  test("shows an empty chat state before a session is selected", async ({ page }) => {
    await page.goto("/");

    await expect(page.getByText("请选择或新建会话")).toBeVisible();
    await expect(page.getByPlaceholder("请选择会话后发送消息")).toBeDisabled();
  });

  test("creates a session, sends a prompt, and renders the mock stream", async ({ page }) => {
    await page.goto("/");

    await createSession(page, "Playwright stream session");

    await page.getByPlaceholder(/输入指令，下发任务给当前 Agent|Enter instructions for the current Agent/).fill("hello from playwright");
    await page.getByRole("button", { name: /发送|Send/ }).click();

    await expect(page.getByText("hello from playwright")).toBeVisible();
    await expect(page.getByText(/Mock .* response|Desktop preview response/)).toBeVisible();
  });

  test("exposes stop while a response is streaming", async ({ page }) => {
    await page.goto("/");

    await createSession(page, "Playwright stop session");
    await page.getByPlaceholder(/输入指令，下发任务给当前 Agent|Enter instructions for the current Agent/).fill("please stream long enough to stop");
    await page.getByRole("button", { name: /发送|Send/ }).click();

    await expect(page.getByRole("button", { name: /停止|Stop/ })).toBeVisible();
  });

  test("keeps messages scoped to the active session", async ({ page }) => {
    await page.goto("/");

    await createSession(page, "Playwright first session");
    await page.getByPlaceholder(/输入指令，下发任务给当前 Agent|Enter instructions for the current Agent/).fill("session one marker");
    await page.getByRole("button", { name: /发送|Send/ }).click();
    await expect(page.getByText("session one marker")).toBeVisible();

    await createSession(page, "Playwright second session");
    await expect(page.getByText("session one marker")).toBeHidden();
  });
});
