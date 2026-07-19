import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("workspace activity bar", () => {
  test("toggles the session sidebar, preserves its view, and keeps panel states independent", async ({ page }) => {
    await page.goto("/");

    const grid = page.locator(".ucd-workspace-grid");
    const sessionSidebar = page.locator("#workspace-session-sidebar");
    const mainPanel = grid.locator(":scope > section").first();
    const archived = page.getByRole("button", { name: /归档/ });
    await archived.click();
    await expect(archived).toHaveClass(/text-primary/);

    const initialBox = await mainPanel.boundingBox();
    expect(initialBox).not.toBeNull();
    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(grid).toHaveAttribute("data-session-collapsed", "true");
    await expect(sessionSidebar).toHaveAttribute("aria-hidden", "true");
    await expect.poll(async () => (await mainPanel.boundingBox())?.width ?? 0).toBeGreaterThan((initialBox?.width ?? 0) + 150);

    await page.getByRole("button", { name: "展开会话栏" }).click();
    await expect(grid).toHaveAttribute("data-session-collapsed", "false");
    await expect(archived).toBeVisible();
    await expect(archived).toHaveClass(/text-primary/);

    await page.getByRole("button", { name: "收起" }).click();
    await expect(grid).toHaveAttribute("data-info-collapsed", "true");
    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(grid).toHaveAttribute("data-session-collapsed", "true");
    await expect(grid).toHaveAttribute("data-info-collapsed", "true");
    await page.getByRole("button", { name: "展开会话栏" }).click();
    await expect(grid).toHaveAttribute("data-info-collapsed", "true");
  });

  test("preserves group mode and expanded folders while keeping collapsed controls inert", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "文件夹状态测试");

    const sessionSidebar = page.locator("#workspace-session-sidebar");
    const groupMode = page.getByRole("button", { name: "分组", exact: true });
    await groupMode.click();
    await expect(groupMode).toHaveClass(/text-primary/);

    const folder = page.getByRole("button", { name: /D:\\example-workspace.*1/ });
    await folder.click();
    const sessionCard = page.getByRole("button", { name: /文件夹状态测试/ });
    await expect(sessionCard).toBeVisible();
    await expect(sessionCard).toHaveClass(/border-primary/);

    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(sessionSidebar).toHaveAttribute("aria-hidden", "true");
    await expect(sessionSidebar).toHaveJSProperty("inert", true);
    await page.keyboard.press("Tab");
    expect(await sessionSidebar.evaluate((element) => element.contains(document.activeElement))).toBe(false);

    await page.getByRole("button", { name: "展开会话栏" }).click();
    await expect(sessionSidebar).toHaveJSProperty("inert", false);
    await expect(groupMode).toHaveClass(/text-primary/);
    await expect(folder).toBeVisible();
    await expect(sessionCard).toBeVisible();
    await expect(sessionCard).toHaveClass(/border-primary/);
  });

  test("opens scheduled tasks and manages a Web mock task", async ({ page }) => {
    await page.goto("/");

    const scheduledTasks = page.getByRole("button", { name: "定时任务" });
    await page.getByRole("button", { name: "折叠会话栏" }).focus();
    await page.keyboard.press("Tab");
    await expect(scheduledTasks).toBeFocused();
    await scheduledTasks.click();
    await expect(page).toHaveURL(/\/workspace$/);
    await expect(page.getByRole("heading", { name: "定时任务" })).toBeVisible();
    await expect(page.getByPlaceholder("例如：每日整理项目进度")).toBeVisible();

    await page.getByLabel("任务名称").fill("每日整理项目进度");
    await page.getByLabel("任务内容").fill("请整理当前项目进度");
    await page.getByLabel("Agent 工具").selectOption("codex-cli");
    await page.getByLabel("执行频率").selectOption("minutes");
    await page.getByRole("spinbutton").fill("15");
    await page.getByRole("button", { name: "创建任务" }).click();

    const taskRow = page.locator(".ucd-list-row").filter({ hasText: "每日整理项目进度" });
    await expect(taskRow).toBeVisible();
    await expect(taskRow.getByText("Codex CLI")).toBeVisible();
    await expect(taskRow.getByText("尚未运行")).toBeVisible();
    await taskRow.getByLabel("已启用").uncheck();
    await expect(taskRow.getByLabel("已停用")).toBeVisible();
    await taskRow.getByLabel("已停用").check();
    await expect(taskRow.getByLabel("已启用")).toBeVisible();

    page.once("dialog", (dialog) => dialog.accept());
    await taskRow.getByRole("button", { name: "删除任务" }).click();
    await expect(page.getByText("每日整理项目进度")).toHaveCount(0);
    await expect(page.getByText("还没有定时任务。")).toBeVisible();
    await expect(page.getByRole("button", { name: "帮助" })).toBeVisible();

    await page.getByRole("button", { name: "关闭定时任务" }).click();
    await expect(page.getByRole("heading", { name: "定时任务" })).toHaveCount(0);
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await expect(page).toHaveURL(/\/settings$/);
  });

  for (const viewport of [
    { name: "900px", width: 900, height: 720 },
    { name: "640px", width: 640, height: 720 },
  ]) {
    test(`keeps the activity bar and session toggle usable at ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto("/");

      const activityBar = page.getByRole("navigation", { name: "工作区导航" });
      const sessionSidebar = page.locator("#workspace-session-sidebar");
      await expect(activityBar).toBeVisible();
      await page.getByRole("button", { name: "折叠会话栏" }).click();
      await expect(sessionSidebar).toHaveAttribute("aria-hidden", "true");
      await page.getByRole("button", { name: "展开会话栏" }).click();
      await expect(sessionSidebar).toHaveAttribute("aria-hidden", "false");
      await expect(activityBar).toBeVisible();
    });
  }
});
