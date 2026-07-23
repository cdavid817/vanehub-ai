import { expect, type Page } from "@playwright/test";

export async function openLoops(page: Page) {
  await page.getByRole("button", { name: "循环工程" }).click();
  await expect(page.locator("#loop-center")).toBeVisible();
}

export async function createAndRunLoop(page: Page, name: string) {
  const create = page.getByRole("button", { name: "新建循环定义" });
  if (!(await create.isVisible())) {
    await page.getByRole("button", { name: "打开循环列表" }).click();
  }
  await create.click();
  await expect(page.getByRole("dialog", { name: "新建循环定义" })).toBeVisible();
  await page.getByLabel("名称").fill(name);
  await page.getByLabel("项目路径").fill("D:\\example-loop-project");
  await page.getByLabel("基础分支").fill("main");
  await page.getByLabel("目标").fill("实现并验证 Loop 工程流程");
  await page.getByLabel("验收标准（每行一项）").fill("所有检查通过\n保留完整证据");
  await page.getByLabel("允许路径（每行一项）").fill("src\ntests");
  await page.getByLabel("保护路径（每行一项）").fill(".git");
  await page.getByRole("button", { name: "下一步" }).click();

  await page.getByLabel("执行智能体").selectOption("codex-cli");
  await page.getByLabel("验证智能体").selectOption("claude-code");
  await page.getByRole("button", { name: "下一步" }).click();
  await expect(page.getByLabel("验证程序")).toHaveValue("npm");
  await page.getByRole("button", { name: "下一步" }).click();
  await expect(page.getByText(name)).toBeVisible();
  await page.getByRole("button", { name: "保存并运行" }).click();
  await expect(page.getByRole("dialog", { name: "新建循环定义" })).toHaveCount(0);

  const closeNavigation = page.getByRole("button", { name: "关闭循环列表" });
  if (await closeNavigation.isVisible()) await closeNavigation.click();
}
