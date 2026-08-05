import { expect, test } from "@playwright/test";

test.describe("Prompt Hook settings", () => {
  test("create and preview-assembly dialogs are real dialogs that close on Escape", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "Prompt Hook" }).click();

    await page.getByRole("button", { name: "新建 Hook" }).click();
    const createDialog = page.getByRole("dialog");
    await expect(createDialog).toBeVisible();
    await expect(createDialog.getByLabel("名称")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(createDialog).toBeHidden();

    await page.getByRole("button", { name: "预览组装" }).click();
    const previewDialog = page.getByRole("dialog");
    await expect(previewDialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(previewDialog).toBeHidden();
  });

  test("edit, advanced lifecycle, and delete dialogs on a user hook are real dialogs", async ({ page }) => {
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

    const card = page.getByRole("listitem").filter({ hasText: "E2E Dialog Hook" });
    await expect(card).toBeVisible();

    await card.getByRole("button", { name: "编辑 Hook" }).click();
    const editDialog = page.getByRole("dialog");
    await expect(editDialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(editDialog).toBeHidden();

    await card.getByRole("button", { name: "打开草稿、版本与效果评估" }).click();
    const lifecycleDialog = page.getByRole("dialog");
    await expect(lifecycleDialog).toBeVisible();
    await expect(lifecycleDialog.getByLabel("模板正文")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(lifecycleDialog).toBeHidden();

    await card.getByRole("button", { name: "删除 Hook" }).click();
    const deleteDialog = page.getByRole("dialog");
    await expect(deleteDialog).toBeVisible();
    await deleteDialog.getByRole("button", { name: "删除", exact: true }).click();
    await expect(deleteDialog).toBeHidden();
    await expect(card).toHaveCount(0);
  });

  test("filter toolbar stays usable at 1024px and the result count sits above the card list", async ({ page }) => {
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

    const showing = page.getByText(/当前显示 \d+ \/ \d+ 个 Prompt Hook/);
    const firstCard = page.getByRole("listitem").first();
    const showingBox = await showing.boundingBox();
    const cardBox = await firstCard.boundingBox();
    expect(showingBox).not.toBeNull();
    expect(cardBox).not.toBeNull();
    expect(showingBox!.y).toBeLessThan(cardBox!.y);
  });
});
