import { expect, test } from "@playwright/test";

test.describe("MCP server settings", () => {
  test("add-server and import/export dialogs are real dialogs that close on Escape", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText(/^(MCP 服务器|MCP Servers)$/).click();

    await page.getByRole("button", { name: /添加 MCP|Add MCP/ }).click();
    const addDialog = page.getByRole("dialog");
    await expect(addDialog).toBeVisible();
    await expect(addDialog.getByRole("textbox").first()).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(addDialog).toBeHidden();

    await page.getByRole("button", { name: /导入\/导出|Import\/Export/ }).click();
    const importExportDialog = page.getByRole("dialog");
    await expect(importExportDialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(importExportDialog).toBeHidden();
  });

  test("masks a saved server's env vars behind an explicit reveal (task 12.13)", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText(/^(MCP 服务器|MCP Servers)$/).click();

    await page.getByRole("button", { name: /添加 MCP|Add MCP/ }).click();
    const addDialog = page.getByRole("dialog");
    await addDialog.getByRole("textbox", { name: /^(名称|Name)$/ }).fill("secret-server");
    await addDialog.getByRole("textbox", { name: /^(命令|Command)$/ }).fill("npx");
    await addDialog.getByRole("textbox", { name: /环境变量 JSON|Env JSON/ }).fill('{"API_KEY": "sk-e2e-secret-value"}');
    await addDialog.getByRole("button", { name: /^(保存|Save)$/ }).click();
    await expect(addDialog).toBeHidden();

    // The nearest `section` ancestor specifically -- `.filter({ has })` would also match an outer
    // wrapping section that contains every card, not just this one.
    const card = page.getByRole("heading", { name: "secret-server", exact: true }).locator("xpath=ancestor::section[1]");
    await card.getByRole("button", { name: /^(编辑|Edit)$/ }).click();
    const editDialog = page.getByRole("dialog");
    await expect(editDialog).toBeVisible();

    // The just-saved secret is not shown just from opening the edit dialog.
    await expect(editDialog.getByRole("textbox", { name: /环境变量 JSON|Env JSON/ })).toHaveCount(0);
    await expect(editDialog.getByRole("button", { name: /^(显示|Reveal)$/ })).toBeVisible();
    await expect(page.getByText("sk-e2e-secret-value")).toHaveCount(0);

    await editDialog.getByRole("button", { name: /^(显示|Reveal)$/ }).click();
    await expect(editDialog.getByRole("textbox", { name: /环境变量 JSON|Env JSON/ })).toHaveValue(/sk-e2e-secret-value/);
  });

  test("server enable/disable toggle has an accessible name", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText(/^(MCP 服务器|MCP Servers)$/).click();

    await expect(page.getByRole("button", { name: /^(禁用|Disable) filesystem-tools$/ }).or(
      page.getByRole("button", { name: /^(启用|Enable) filesystem-tools$/ }),
    )).toBeVisible();
  });
});
