import { expect, test } from "@playwright/test";

test.describe("Skills settings page", () => {
  test("renders scope controls, Skill cards, and Agent mount paths", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /Skill 管理|Skills/ }).click();

    await expect(page.getByText(/Agent 挂载路径|Agent mount paths/)).toBeVisible();
    await expect(page.getByRole("button", { name: /全局|Global/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /工作区|Workspace/ })).toBeVisible();
    await expect(page.getByText("TDD 开发纪律助手")).toBeVisible();
  });

  test("supports workspace mode, filtering, and restore dialog entry", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /Skill 管理|Skills/ }).click();
    await page.getByRole("button", { name: /工作区|Workspace/ }).click();
    await page.getByPlaceholder(/选择本地项目目录|Select local project directory/).fill("D:\\example-workspace");
    await page.getByPlaceholder(/按 ID、名称、分类、触发词或来源搜索|Search by id, name, category, trigger, or source/).fill("readme");
    await page.getByRole("button", { name: /恢复内置|Restore Built-in/ }).click();

    await expect(page.getByText(/恢复内置 Skill|Restore built-in Skill/)).toBeVisible();
  });
});
