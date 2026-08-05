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

  test("server enable/disable toggle has an accessible name", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText(/^(MCP 服务器|MCP Servers)$/).click();

    await expect(page.getByRole("button", { name: /^(禁用|Disable) filesystem-tools$/ }).or(
      page.getByRole("button", { name: /^(启用|Enable) filesystem-tools$/ }),
    )).toBeVisible();
  });
});
