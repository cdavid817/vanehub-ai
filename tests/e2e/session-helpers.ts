import { expect, type Page } from "@playwright/test";

export async function createSession(page: Page, title: string) {
  await page.getByRole("button", { name: /新建/ }).click();
  const projectPath = page.getByPlaceholder(/code.*project/);
  const sessionTitle = page.getByPlaceholder("新会话");
  const createButton = page.getByRole("button", { name: "创建", exact: true });

  await expect(async () => {
    await projectPath.fill("D:\\example-workspace");
    await projectPath.press("Tab");
    await sessionTitle.fill(title);
    await expect(createButton).toBeEnabled({ timeout: 1_000 });
  }).toPass({ timeout: 10_000 });

  await createButton.click();
  await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled();
}
