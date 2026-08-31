import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

const primaryTabNames = ["工作区", "变更", "文件", "报告"];
const runtimeTabNames = ["终端记录", "Shell", "日志", "链路"];

function primaryTablist(page: Parameters<typeof createSession>[0]) {
  return page.getByRole("tablist", { name: "会话工作区" });
}

function runtimeTablist(page: Parameters<typeof createSession>[0]) {
  return page.getByRole("tablist", { name: "运行时面板" });
}

function activeWorkspacePanel(page: Parameters<typeof createSession>[0]) {
  return page.locator('[id^="session-tab-panel-"]:not(.hidden)');
}

async function openWorkspace(page: Parameters<typeof createSession>[0], title = "工作区标签测试") {
  await page.goto("/");
  await createSession(page, title);
}

async function openRuntimePanel(page: Parameters<typeof createSession>[0]) {
  await page.getByRole("button", { name: "运行时面板" }).click();
  await expect(runtimeTablist(page)).toBeVisible();
}

test.describe("session workspace tabs", () => {
  test("exposes four accessible primary tabs and supports keyboard navigation", async ({ page }) => {
    await openWorkspace(page);

    const tabs = primaryTablist(page);
    await expect(tabs.getByRole("tab")).toHaveCount(4);
    for (const name of primaryTabNames) await expect(tabs.getByRole("tab", { name })).toBeVisible();

    const workspace = tabs.getByRole("tab", { name: "工作区" });
    await workspace.focus();
    await workspace.press("ArrowRight");
    await expect(tabs.getByRole("tab", { name: "变更" })).toHaveAttribute("aria-selected", "true");
    await tabs.getByRole("tab", { name: "变更" }).press("End");
    await expect(tabs.getByRole("tab", { name: "报告" })).toBeFocused();
    await expect(tabs.getByRole("tab", { name: "报告" })).toHaveAttribute("aria-selected", "true");
    await tabs.getByRole("tab", { name: "报告" }).press("Home");
    await expect(workspace).toBeFocused();
  });

  test("exposes four accessible Runtime Panel tabs once opened, with their own keyboard navigation", async ({ page }) => {
    await openWorkspace(page);
    await openRuntimePanel(page);

    const tabs = runtimeTablist(page);
    await expect(tabs.getByRole("tab")).toHaveCount(4);
    for (const name of runtimeTabNames) await expect(tabs.getByRole("tab", { name })).toBeVisible();

    const terminalHistory = tabs.getByRole("tab", { name: "终端记录" });
    await expect(terminalHistory).toHaveAttribute("aria-selected", "true");
    await terminalHistory.focus();
    await terminalHistory.press("ArrowRight");
    await expect(tabs.getByRole("tab", { name: "Shell" })).toHaveAttribute("aria-selected", "true");

    // The trigger only exists while the panel is closed; opening it is a one-way door until Close.
    await expect(page.getByRole("button", { name: "运行时面板" })).toHaveCount(0);
    await page.getByRole("button", { name: "关闭", exact: true }).click();
    await expect(tabs).toHaveCount(0);
    await expect(page.getByRole("button", { name: "运行时面板" })).toBeVisible();
  });

  test("keeps the folder opener outside the tablist and exposes deterministic Web options", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openWorkspace(page, "文件夹打开方式测试");

    await expect(primaryTablist(page).getByRole("tab")).toHaveCount(4);
    await expect(page.getByRole("button", { name: /使用 Visual Studio Code 打开文件夹/ })).toBeVisible();
    await page.getByRole("button", { name: "选择工作区打开工具" }).click();
    await expect(page.getByRole("menuitem", { name: /Visual Studio Code/ })).toBeVisible();
    await expect(page.getByRole("menuitem", { name: /Visual Studio Code/ })).toBeFocused();
    await expect(page.getByRole("menuitem", { name: /文件资源管理器/ })).toBeVisible();
    await expect(page.getByRole("menuitem", { name: /Windows Terminal/ })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    await page.getByRole("menuitem", { name: /Visual Studio Code/ }).press("Escape");
    await expect(page.getByRole("button", { name: "选择工作区打开工具" })).toBeFocused();
    await page.getByRole("button", { name: "选择工作区打开工具" }).click();
    await page.getByRole("menuitem", { name: "管理工作区打开工具" }).click();
    await expect(page.getByRole("heading", { name: "工作区", exact: true })).toBeVisible();
    await page.getByText("管理工作区打开工具", { exact: true }).click();
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

  test("switches Files between its Explorer and Documents subviews without losing either's state", async ({ page }) => {
    await openWorkspace(page);

    await primaryTablist(page).getByRole("tab", { name: "文件" }).click();
    const filesSwitcher = page.getByRole("tablist", { name: "文件视图" });
    await expect(filesSwitcher.getByRole("tab", { name: "资源管理器" })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("button", { name: /README\.md/ })).toBeVisible();

    await filesSwitcher.getByRole("tab", { name: "文档" }).click();
    await expect(page.getByRole("heading", { name: "VaneHub Web Preview" })).toBeVisible();

    await filesSwitcher.getByRole("tab", { name: "资源管理器" }).click();
    await expect(page.getByRole("button", { name: /README\.md/ })).toBeVisible();
  });

  test("keeps primary and Runtime Panel state independent while switching either", async ({ page }) => {
    await openWorkspace(page);
    const composer = page.getByRole("textbox", { name: "工作区命令输入" });
    await composer.fill("保留这个草稿");

    await openRuntimePanel(page);
    const runtimeTabs = runtimeTablist(page);
    await runtimeTabs.getByRole("tab", { name: "日志" }).click();
    const search = page.getByRole("textbox", { name: "搜索脱敏日志" });
    await search.fill("runtime");
    await search.press("Enter");
    await expect(page.getByText("Web preview session initialized.")).toBeVisible();

    // Within-panel switching preserves state (the Runtime Panel's own retained-tab contract).
    await runtimeTabs.getByRole("tab", { name: "链路" }).click();
    await runtimeTabs.getByRole("tab", { name: "日志" }).click();
    await expect(search).toHaveValue("runtime");

    // Switching the *primary* surface does not touch the Runtime Panel at all — they are two
    // independent regions now, not two tabs on the shared tablist they used to be.
    await primaryTablist(page).getByRole("tab", { name: "报告" }).click();
    await expect(composer).toBeHidden();
    await expect(search).toHaveValue("runtime");

    await primaryTablist(page).getByRole("tab", { name: "工作区" }).click();
    await expect(composer).toHaveValue("保留这个草稿");
  });

  test("renders deterministic Web fixtures for project and operational surfaces", async ({ page }) => {
    await openWorkspace(page);

    await primaryTablist(page).getByRole("tab", { name: "文件" }).click();
    await page.getByRole("button", { name: /README\.md/ }).click();
    await expect(page.getByText("VaneHub Web Preview")).toBeVisible();

    await primaryTablist(page).getByRole("tab", { name: "变更" }).click();
    await expect(page.getByText("worktree/web-preview")).toBeVisible();
    await expect(page.getByText("export const runtime = \"web-mock\";")).toBeVisible();
    await page.getByRole("button", { name: "分栏视图" }).click();

    await openRuntimePanel(page);
    const runtimeTabs = runtimeTablist(page);
    await runtimeTabs.getByRole("tab", { name: "日志" }).click();
    await expect(page.getByText("Loaded deterministic project fixtures.")).toBeVisible();
    await page.getByRole("button", { name: "导出" }).click();
    await expect(page.getByText("Web 预览模式不支持导出本地日志。")).toBeVisible();

    await runtimeTabs.getByRole("tab", { name: "Shell" }).click();
    await expect(page.getByRole("tabpanel", { name: "Shell" }).getByText("模拟环境", { exact: true })).toBeVisible();
    await expect(page.getByLabel("会话交互式 Shell")).toBeVisible();
  });

  test("shows simulated Agent terminal input and a report that states its own coverage", async ({ page }) => {
    await openWorkspace(page);
    await page.getByRole("textbox", { name: "工作区命令输入" }).fill("echo workspace data");
    await page.getByRole("button", { name: "发送命令" }).click();
    await expect(page.getByLabel("Agent CLI 工作区")).toContainText("echo workspace data");

    await openRuntimePanel(page);
    const terminalHistory = runtimeTablist(page).getByRole("tab", { name: "终端记录" });
    await expect(terminalHistory).not.toContainText("0");
    // Terminal History reads the execution record query rather than the loaded messages, so the
    // deterministic native fixtures are what it shows. The legacy message-history projection lives
    // behind its own view, and session-workspace-terminal-history.spec.ts walks both.
    await expect(page.getByRole("tabpanel", { name: "终端记录" })).toContainText("npm test");

    await primaryTablist(page).getByRole("tab", { name: "报告" }).click();
    const report = activeWorkspacePanel(page);
    // Report reads the backend aggregate now rather than whatever messages happen to be mounted,
    // so it is no longer empty here — and what makes it honest has moved with it. It is no longer
    // the empty state but what the report says about figures it cannot substantiate: coverage is
    // per section, so a section nothing backs says so while the rest still reads as complete.
    await expect(report).toContainText("总览");
    await expect(report).toContainText("完整");
    // The mock has no usage accounting behind it, and the usage section says that rather than
    // presenting the numbers it does have as though the set were whole.
    await expect(report).toContainText("部分");
    // Absent, not zero. A reported zero would mean somebody reported zero tokens; nobody did, and
    // the two are different claims that a dash keeps apart and a `0` would not.
    await expect(report).toContainText("上报输入 token");
    await expect(report).not.toContainText("上报输入 token0");
  });

  test("resets mounted primary tabs and closes the Runtime Panel when selecting another session", async ({ page }) => {
    await openWorkspace(page, "第一会话");
    await primaryTablist(page).getByRole("tab", { name: "文件" }).click();
    await expect(primaryTablist(page).getByRole("tab", { name: "文件" })).toHaveAttribute("aria-selected", "true");
    await openRuntimePanel(page);
    await expect(runtimeTablist(page)).toBeVisible();

    await createSession(page, "第二会话");
    await expect(primaryTablist(page).getByRole("tab", { name: "工作区" })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("textbox", { name: "工作区命令输入" })).toBeVisible();
    await expect(page.locator('[id^="session-tab-panel-"]')).toHaveCount(1);
    // The panel does not silently carry the previous session's Files/Shell state into this one —
    // it closes, and the reader who wants it back opens it fresh against the new session.
    await expect(runtimeTablist(page)).toHaveCount(0);
    await expect(page.getByRole("button", { name: "运行时面板" })).toBeVisible();
  });

  for (const variant of [
    { theme: "futuristic", width: 1440, height: 900 },
    { theme: "minimal", width: 390, height: 844 },
  ]) {
    test(`keeps the primary tab bar and Runtime Panel usable in ${variant.theme} at ${variant.width}px`, async ({ page }) => {
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

      for (const name of primaryTabNames) {
        await primaryTablist(page).getByRole("tab", { name }).click();
        await expect(primaryTablist(page).getByRole("tab", { name })).toHaveAttribute("aria-selected", "true");
        await expect(activeWorkspacePanel(page)).toBeVisible();
      }
      await openRuntimePanel(page);
      for (const name of runtimeTabNames) {
        await runtimeTablist(page).getByRole("tab", { name }).click();
        await expect(runtimeTablist(page).getByRole("tab", { name })).toHaveAttribute("aria-selected", "true");
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    });
  }
});
