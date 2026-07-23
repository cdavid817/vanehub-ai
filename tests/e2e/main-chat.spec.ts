import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("main Agent workspace experience", () => {
  test("shows an unavailable workspace state before a session is selected", async ({ page }) => {
    await page.goto("/");

    await expect(page.getByRole("tabpanel", { name: "工作区" })).toContainText(
      "请选择具有可用工作目录的会话后使用此标签页。",
    );
    await expect(page.getByRole("button", { name: "请先选择一个会话。" })).toBeDisabled();
  });

  test("creates a session and sends input to the simulated Agent terminal", async ({ page }) => {
    await page.goto("/");

    await createSession(page, "Playwright 会话");

    await page.getByRole("textbox", { name: "工作区命令输入" }).fill("hello from playwright");
    await page.getByRole("button", { name: "发送命令" }).click();

    await expect(page.getByLabel("Agent CLI 工作区")).toContainText("hello from playwright");
    await expect(page.getByText("模拟环境", { exact: true })).toBeVisible();
  });

  test("exposes stop while the Agent terminal is connected", async ({ page }) => {
    await page.goto("/");

    await createSession(page, "停止终端测试");
    const stop = page.getByRole("button", { name: "停止", exact: true });
    await expect(stop).toBeEnabled();
    await stop.click();

    await expect(
      page.getByLabel("工作区", { exact: true }).getByText("已停止", { exact: true }),
    ).toBeVisible();
    await expect(page.getByRole("textbox", { name: "工作区命令输入" })).toBeDisabled();
  });

  test("keeps terminal input scoped to the active session", async ({ page }) => {
    await page.goto("/");

    await createSession(page, "会话一");
    await page.getByRole("textbox", { name: "工作区命令输入" }).fill("session one marker");
    await page.getByRole("button", { name: "发送命令" }).click();
    await expect(page.getByLabel("Agent CLI 工作区")).toContainText("session one marker");

    await createSession(page, "会话二");
    await expect(page.getByRole("textbox", { name: "工作区命令输入" })).toHaveValue("");
    await expect(page.getByLabel("Agent CLI 工作区")).not.toContainText("session one marker");
  });
});
