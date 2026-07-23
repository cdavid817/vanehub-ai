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

  test("resizes the session sidebar and restores the persisted width after collapse and reload", async ({ page }) => {
    await page.addInitScript(() => {
      if (window.localStorage.getItem("vanehub.session-sidebar.width.v1") === null) {
        window.localStorage.setItem("vanehub.session-sidebar.width.v1", "300");
      }
    });
    await page.goto("/");

    const sessionSidebar = page.locator("#workspace-session-sidebar");
    const resizeHandle = page.getByRole("button", { name: "调整会话栏宽度" });
    await expect.poll(async () => Math.round((await sessionSidebar.boundingBox())?.width ?? 0)).toBe(300);

    const handleBox = await resizeHandle.boundingBox();
    expect(handleBox).not.toBeNull();
    if (!handleBox) throw new Error("Session sidebar resize handle is not visible.");
    await page.mouse.move(handleBox.x + handleBox.width / 2, handleBox.y + handleBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(handleBox.x + handleBox.width / 2 + 90, handleBox.y + handleBox.height / 2);
    await page.mouse.up();

    await expect.poll(async () => Math.round((await sessionSidebar.boundingBox())?.width ?? 0)).toBeGreaterThan(380);
    const persistedWidth = await page.evaluate(() => Number(window.localStorage.getItem("vanehub.session-sidebar.width.v1")));
    expect(persistedWidth).toBeGreaterThan(380);
    expect(persistedWidth).toBeLessThanOrEqual(420);

    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(sessionSidebar).toHaveJSProperty("inert", true);
    await page.getByRole("button", { name: "展开会话栏" }).click();
    await expect(sessionSidebar).toHaveJSProperty("inert", false);
    await expect.poll(async () => Math.round((await sessionSidebar.boundingBox())?.width ?? 0)).toBe(persistedWidth);

    await page.reload();
    await expect.poll(async () => Math.round((await sessionSidebar.boundingBox())?.width ?? 0)).toBe(persistedWidth);
  });

  test("restores project grouping preferences and keeps the active session while toggling a group", async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("vanehub.session-sidebar.presentation.v1", "project");
      window.localStorage.setItem(
        "vanehub.session-sidebar.expanded-groups.v1",
        JSON.stringify(["project:D:\\example-workspace"]),
      );
    });
    await page.goto("/");
    await createSession(page, "文件夹状态测试");

    const sessionSidebar = page.locator("#workspace-session-sidebar");
    const projectMode = page.getByRole("button", { name: "项目", exact: true });
    await expect(projectMode).toHaveClass(/text-primary/);

    const folder = page.getByRole("button", { name: /example-workspace.*1/ });
    const sessionCard = page.getByRole("button", { name: /文件夹状态测试/ });
    await expect(sessionCard).toBeVisible();
    await expect(sessionCard).toHaveClass(/border-primary/);

    await folder.click();
    await expect(sessionCard).toBeHidden();
    expect(await page.evaluate(() => window.localStorage.getItem("vanehub.session-sidebar.expanded-groups.v1"))).toBe("[]");
    await folder.click();
    await expect(sessionCard).toBeVisible();
    await expect(sessionCard).toHaveClass(/border-primary/);
    expect(await page.evaluate(() => JSON.parse(window.localStorage.getItem("vanehub.session-sidebar.expanded-groups.v1") ?? "[]"))).toContain("project:D:\\example-workspace");

    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(sessionSidebar).toHaveAttribute("aria-hidden", "true");
    await expect(sessionSidebar).toHaveJSProperty("inert", true);
    await page.keyboard.press("Tab");
    expect(await sessionSidebar.evaluate((element) => element.contains(document.activeElement))).toBe(false);

    await page.getByRole("button", { name: "展开会话栏" }).click();
    await expect(sessionSidebar).toHaveJSProperty("inert", false);
    await expect(projectMode).toHaveClass(/text-primary/);
    await expect(folder).toBeVisible();
    await expect(sessionCard).toBeVisible();
    await expect(sessionCard).toHaveClass(/border-primary/);
  });

  test("opens scheduled tasks and manages a Web mock task", async ({ page }) => {
    await page.goto("/");

    const scheduledTasks = page.getByRole("button", { name: "定时任务" });
    await page.getByRole("button", { name: "折叠会话栏" }).focus();
    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: "循环工程" })).toBeFocused();
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
