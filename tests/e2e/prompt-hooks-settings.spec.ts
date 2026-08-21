import { expect, test } from "@playwright/test";

test.describe("Prompt Hook settings", () => {
  test("create and runtime preview dialogs are real dialogs that close on Escape", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "Prompt Hook" }).click();

    await page.getByRole("button", { name: "新建 Hook" }).click();
    const createDialog = page.getByRole("dialog");
    await expect(createDialog).toBeVisible();
    await expect(createDialog.getByLabel("名称")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(createDialog).toBeHidden();

    await page.getByRole("tab", { name: "运行记录" }).click();
    await page.getByRole("button", { name: "预览组装" }).click();
    const previewDialog = page.getByRole("dialog");
    await expect(previewDialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(previewDialog).toBeHidden();
  });

  test("one user-hook detail dialog owns editing, lifecycle, and deletion", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "Prompt Hook" }).click();

    await page.getByRole("button", { name: "新建 Hook" }).click();
    const createDialog = page.getByRole("dialog");
    await createDialog.getByLabel("ID", { exact: true }).fill("e2e-dialog-hook");
    await createDialog.getByLabel("名称").fill("E2E Dialog Hook");
    await createDialog.getByLabel("描述").fill("e2e verification hook");
    await createDialog.getByRole("button", { name: "保存" }).click();
    await expect(createDialog).toBeHidden();

    const row = page.getByRole("listitem").filter({ hasText: "E2E Dialog Hook" });
    await expect(row).toBeVisible();

    await row.getByRole("button", { name: "打开 E2E Dialog Hook 的详情" }).click();
    const detailDialog = page.getByRole("dialog");
    await expect(detailDialog).toBeVisible();
    await expect(detailDialog.getByRole("tab", { name: "基本设置" })).toBeFocused();
    await detailDialog.getByRole("tab", { name: "内容与发布" }).click();
    await expect(detailDialog.getByLabel("模板正文")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(detailDialog).toBeHidden();

    await row.getByRole("button", { name: "打开 E2E Dialog Hook 的详情" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "删除", exact: true }).click();
    const deleteDialog = page.getByRole("dialog");
    await expect(deleteDialog).toBeVisible();
    await deleteDialog.getByRole("button", { name: "删除", exact: true }).click();
    await expect(deleteDialog).toBeHidden();
    await expect(row).toHaveCount(0);
  });

  test("filter toolbar stays usable at 1024px and the compact summary sits above the list", async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "Prompt Hook" }).click();

    const search = page.getByPlaceholder("按 ID、名称、描述、分类或来源搜索");
    await expect(search).toBeVisible();
    const searchBox = await search.boundingBox();
    expect(searchBox).not.toBeNull();
    expect(searchBox!.width).toBeGreaterThan(300);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    const summary = page.getByText(/显示 \d+ \/ \d+ · 已启用 \d+ · 自定义 \d+/);
    const firstRow = page.getByRole("listitem").first();
    const summaryBox = await summary.boundingBox();
    const rowBox = await firstRow.boundingBox();
    expect(summaryBox).not.toBeNull();
    expect(rowBox).not.toBeNull();
    expect(summaryBox!.y).toBeLessThan(rowBox!.y);
  });
});
