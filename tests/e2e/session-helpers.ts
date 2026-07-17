import { expect, type Page } from "@playwright/test";

export async function createSession(page: Page, title: string) {
  await page.getByRole("button", { name: /新建/ }).click();
  const projectPath = page.getByPlaceholder(/code.*project/);
  await projectPath.fill("D:\\example-workspace");
  await projectPath.press("Tab");
  await page.getByPlaceholder("新会话").fill(title);
  await page.getByRole("button", { name: "创建", exact: true }).click();
  await expect(page.getByPlaceholder("输入指令，下发任务给当前 Agent...")).toBeEnabled();
}
