import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

const tabNames = ["聊天", "变更", "文档", "文件", "终端记录", "Shell", "日志", "链路", "报告"];

async function openWorkspace(page: Parameters<typeof createSession>[0], title = "工作区标签测试") {
  await page.goto("/");
  await createSession(page, title);
}

test.describe("session workspace tabs", () => {
  test("exposes nine accessible tabs and supports keyboard navigation", async ({ page }) => {
    await openWorkspace(page);

    await expect(page.getByRole("tab")).toHaveCount(9);
    for (const name of tabNames) await expect(page.getByRole("tab", { name })).toBeVisible();

    const chat = page.getByRole("tab", { name: "聊天" });
    await chat.focus();
    await chat.press("ArrowRight");
    await expect(page.getByRole("tab", { name: "变更" })).toHaveAttribute("aria-selected", "true");
    await page.getByRole("tab", { name: "变更" }).press("End");
    await expect(page.getByRole("tab", { name: "报告" })).toBeFocused();
    await expect(page.getByRole("tab", { name: "报告" })).toHaveAttribute("aria-selected", "true");
    await page.getByRole("tab", { name: "报告" }).press("Home");
    await expect(chat).toBeFocused();
  });

  test("keeps the folder opener outside the tablist and exposes deterministic Web options", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openWorkspace(page, "文件夹打开方式测试");

    await expect(page.getByRole("tab")).toHaveCount(9);
    await expect(page.getByRole("button", { name: /使用 Visual Studio Code 打开文件夹/ })).toBeVisible();
    await page.getByRole("button", { name: "选择文件夹打开方式" }).click();
    await expect(page.getByRole("menuitem", { name: /Visual Studio Code/ })).toBeVisible();
    await expect(page.getByRole("menuitem", { name: /Visual Studio Code/ })).toBeFocused();
    await expect(page.getByRole("menuitem", { name: /文件资源管理器/ })).toBeVisible();
    await expect(page.getByRole("menuitem", { name: /Windows Terminal/ })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    await page.getByRole("menuitem", { name: /Visual Studio Code/ }).press("Escape");
    await expect(page.getByRole("button", { name: "选择文件夹打开方式" })).toBeFocused();
    await page.getByRole("button", { name: "选择文件夹打开方式" }).click();
    await page.getByRole("menuitem", { name: "管理打开方式" }).click();
    await expect(page.getByRole("heading", { name: "文件夹打开方式" })).toBeVisible();
    const gitBash = page.getByRole("checkbox", { name: /Git Bash/ });
    await expect(gitBash).toBeChecked();
    await gitBash.uncheck();
    await expect(gitBash).not.toBeChecked();
    await expect(page.getByRole("checkbox", { name: /文件资源管理器/ })).toBeDisabled();
  });

  test("reports the Web native-launch limitation without claiming success", async ({ page }) => {
    await openWorkspace(page, "Web 打开限制测试");
    await page.getByRole("button", { name: /使用 Visual Studio Code 打开文件夹/ }).click();
    await expect(page.getByRole("button", { name: "Web 预览模式不能启动本地程序。" })).toBeVisible();
  });

  test("keeps mounted tab state and chat draft while switching tabs", async ({ page }) => {
    await openWorkspace(page);
    const composer = page.getByPlaceholder("输入指令，下发任务给当前 Agent...");
    await composer.fill("保留这个草稿");

    await page.getByRole("tab", { name: "日志" }).click();
    const search = page.getByRole("textbox", { name: "搜索脱敏日志" });
    await search.fill("runtime");
    await search.press("Enter");
    await expect(page.getByText("Web preview session initialized.")).toBeVisible();

    await page.getByRole("tab", { name: "报告" }).click();
    await expect(composer).toBeHidden();
    await page.getByRole("tab", { name: "日志" }).click();
    await expect(search).toHaveValue("runtime");
    await page.getByRole("tab", { name: "聊天" }).click();
    await expect(composer).toHaveValue("保留这个草稿");
  });

  test("renders deterministic Web fixtures for project and operational tabs", async ({ page }) => {
    await openWorkspace(page);

    await page.getByRole("tab", { name: "文件" }).click();
    await page.getByRole("button", { name: /README\.md/ }).click();
    await expect(page.getByText("VaneHub Web Preview")).toBeVisible();

    await page.getByRole("tab", { name: "文档" }).click();
    await expect(page.getByRole("heading", { name: "VaneHub Web Preview" })).toBeVisible();

    await page.getByRole("tab", { name: "变更" }).click();
    await expect(page.getByText("worktree/web-preview")).toBeVisible();
    await expect(page.getByText("export const runtime = \"web-mock\";")).toBeVisible();
    await page.getByRole("button", { name: "分栏视图" }).click();

    await page.getByRole("tab", { name: "日志" }).click();
    await expect(page.getByText("Loaded deterministic project fixtures.")).toBeVisible();
    await page.getByRole("button", { name: "导出" }).click();
    await expect(page.getByText("Web 预览模式不支持导出本地日志。")).toBeVisible();

    await page.getByRole("tab", { name: "Shell" }).click();
    await expect(page.getByText("模拟环境", { exact: true })).toBeVisible();
    await expect(page.getByLabel("会话交互式 Shell")).toBeVisible();
  });

  test("shows Terminal badge/cards and Report after a mock response", async ({ page }) => {
    await openWorkspace(page);
    await page.getByPlaceholder("输入指令，下发任务给当前 Agent...").fill("生成工具与报告数据");
    await page.getByRole("button", { name: "发送" }).click();

    const terminal = page.getByRole("tab", { name: /终端记录/ });
    await expect(terminal).toContainText("1");
    await terminal.click();
    await expect(page.getByRole("heading", { name: "read_file" })).toBeVisible();
    await expect(page.locator('[role="tabpanel"]:not(.hidden)')).toContainText("README.md");

    await page.getByRole("tab", { name: "报告" }).click();
    await expect(page.getByText("工具排行")).toBeVisible();
    await expect(page.getByText("read_file").last()).toBeVisible();
    await expect(page.getByText("已报告输入 Token")).toBeVisible();
  });

  test("resets mounted tabs and active tab when selecting another session", async ({ page }) => {
    await openWorkspace(page, "第一会话");
    await page.getByRole("tab", { name: "文件" }).click();
    await expect(page.getByRole("tab", { name: "文件" })).toHaveAttribute("aria-selected", "true");

    await createSession(page, "第二会话");
    await expect(page.getByRole("tab", { name: "聊天" })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByPlaceholder("输入指令，下发任务给当前 Agent...")).toBeVisible();
    await expect(page.getByRole("tabpanel")).toHaveCount(1);
  });

  for (const variant of [
    { theme: "futuristic", width: 1440, height: 900 },
    { theme: "minimal", width: 390, height: 844 },
  ]) {
    test(`keeps the tab bar usable in ${variant.theme} at ${variant.width}px`, async ({ page }) => {
      await page.setViewportSize({ width: variant.width, height: variant.height });
      await page.addInitScript((theme) => {
        window.localStorage.setItem(
          "vanehub.appSettings",
          JSON.stringify({ applicationLanguage: "zh-CN", theme }),
        );
      }, variant.theme);
      await page.goto("/");
      await createSession(page, `${variant.theme} 主题`);
      await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
      for (const name of tabNames) {
        await page.getByRole("tab", { name }).click();
        await expect(page.getByRole("tab", { name })).toHaveAttribute("aria-selected", "true");
        await expect(page.locator('[role="tabpanel"]:not(.hidden)')).toBeVisible();
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    });
  }
});
