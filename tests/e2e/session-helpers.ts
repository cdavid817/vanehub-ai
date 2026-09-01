import { expect, type Page } from "@playwright/test";

/**
 * Task 11.3-11.7 turned the single-screen create-session dialog into a 4-step wizard: Step 1
 * (mode), Step 2 (participant), Step 3 (workspace — the project path field lives here now, not
 * on the first screen), Step 4 (review, including the session name field, since Step 4 is where
 * the old single-screen dialog's own separate "Identity" section landed). Every helper that used
 * to fill the path and the title on one screen now has to click through the intervening steps
 * first; defaults on Steps 1-2 are left as-is (single-Agent, whatever Agent/mode the reset action
 * already pre-selects), matching what a reader doing nothing beyond "create a session" would do.
 */
export async function createSession(page: Page, title: string) {
  await page.getByRole("button", { name: /新建/ }).click();
  const nextButton = page.getByRole("button", { name: "下一步" });
  await nextButton.click(); // Step 1 (mode) -> Step 2, defaults left as-is.
  await nextButton.click(); // Step 2 (participant) -> Step 3, defaults left as-is.

  const projectPath = page.getByPlaceholder(/code.*project/);
  await projectPath.fill("D:\\example-workspace");
  await projectPath.press("Tab");
  // Next only enables once the async project-path validation this same fill triggers settles.
  await expect(nextButton).toBeEnabled({ timeout: 10_000 });
  await nextButton.click(); // Step 3 (workspace) -> Step 4 (review).

  await page.getByPlaceholder("新会话").fill(title);
  const createButton = page.getByRole("button", { name: "创建", exact: true });
  await expect(createButton).toBeEnabled();
  await createButton.click();
  await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled();
}
