import { expect, test, type Locator, type Page } from "@playwright/test";

function settingsSection(page: Page, title: string): Locator {
  return page.getByRole("heading", { name: title, exact: true })
    .locator("xpath=ancestor::section[1]");
}

async function openLspSettings(page: Page) {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(代码智能|Code Intelligence)$/ }).click();
  await expect(page.getByRole("heading", {
    name: /^(代码智能|Code Intelligence)$/,
    level: 2,
  })).toBeVisible();
  await expect(page.getByRole("heading", {
    name: "语言服务器智能",
    exact: true,
  })).toBeVisible();
}

test.describe("LSP settings in Web mode", () => {
  test("configures, validates, trusts, tests, transitions, and revokes", async ({ page }) => {
    await openLspSettings(page);

    const configuration = settingsSection(page, "语言服务器智能");
    const rustOptions = configuration.getByRole("textbox", { name: "Rust · 初始化选项" });
    await configuration.getByRole("checkbox", { name: /^启用 LSP 集成/ }).check();
    await configuration.getByRole("checkbox", { name: "启用 Rust 语言服务器" }).check();
    await rustOptions.fill("{broken");
    await configuration.getByRole("button", { name: "保存 LSP 配置" }).click();
    await expect(configuration.getByRole("alert")).toHaveText("请输入有效的 JSON。");
    await expect(rustOptions).toHaveAttribute("aria-invalid", "true");

    await rustOptions.fill('{"cargo":{"allTargets":true}}');
    await configuration.getByRole("button", { name: "保存 LSP 配置" }).click();
    await expect(configuration.getByRole("status")).toContainText("LSP 配置已保存。");

    const runtime = settingsSection(page, "运行状态");
    await expect(runtime.getByText("当前没有活动的语言服务器实例。")).toBeVisible();
    const trust = settingsSection(page, "受信任的工作区");
    const workspaceRoot = "D:/code/playwright-lsp";
    await trust.getByRole("textbox", { name: "本地工作区绝对路径" }).fill(workspaceRoot);
    await trust.getByRole("button", { name: "信任工作区" }).click();
    await expect(trust.getByText(workspaceRoot, { exact: true })).toBeVisible();

    const rustStatus = runtime.getByRole("article", { name: "Rust rust_analyzer" });
    await expect(rustStatus.getByText("正在启动", { exact: true })).toBeVisible();
    const refreshStatus = runtime.getByRole("button", { name: "刷新状态" });
    await refreshStatus.click();
    await expect(rustStatus.getByText("正在初始化", { exact: true })).toBeVisible();
    await refreshStatus.click();
    await expect(rustStatus.getByText("就绪", { exact: true })).toBeVisible();
    await expect(rustStatus.getByText("UTF-16", { exact: true })).toBeVisible();

    const serverTest = settingsSection(page, "测试语言服务器");
    await serverTest.getByRole("button", { name: /测试服务器.*Rust/ }).click();
    const testResult = serverTest.getByRole("status");
    await expect(testResult).toContainText("语言服务器测试成功。");
    await expect(testResult.getByText("成功", { exact: true })).toHaveCount(4);

    await trust.getByRole("button", { name: `撤销信任 ${workspaceRoot}` }).click();
    await expect(trust.getByText("尚未信任任何工作区使用 LSP。")).toBeVisible();
    await expect(runtime.getByText("当前没有活动的语言服务器实例。")).toBeVisible();
  });
});
