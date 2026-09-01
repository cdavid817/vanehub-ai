import { expect, test, type Page } from "@playwright/test";

/**
 * The mode a session is created with, and the badge that keeps saying so.
 *
 * A mode is a promise made once, at creation, about what a conversation will and will not use.
 * These check that the promise reaches the store and stays visible while the conversation is open:
 * a user who cannot see it has no way to know it is still in force.
 *
 * Surviving a restart is not checked here. The Web/mock adapter keeps sessions in module memory, so
 * a reload starts from nothing -- that is the mock's own shape, not the behaviour under test, and
 * asserting it here would only prove the mock forgets. Restart belongs to the desktop layer, where
 * sessions are actually persisted.
 *
 * Task 11.3-11.7 turned the single-screen create-session dialog into a 4-step wizard: Step 1
 * (mode), Step 2 (participant, including the personalization-mode selector this whole spec is
 * about — moved here from the old single-screen dialog's "Workspace" section), Step 3 (workspace,
 * the project path field), Step 4 (review, including the session name field). "Opening the
 * dialog" below always lands on Step 2, since every test in this file needs that selector.
 */
async function openCreateDialog(page: Page) {
  await page.goto("/");
  await reopenCreateDialog(page);
}

/** Opens the dialog again without navigating, which would take the mock's sessions with it.
 *  Lands on Step 2 (defaults from Step 1 left as-is), where personalization mode lives now. */
async function reopenCreateDialog(page: Page) {
  await page.getByRole("button", { name: /新建/ }).click();
  await page.getByRole("button", { name: "下一步" }).click(); // Step 1 -> Step 2.
  await expect(page.getByTestId("session-personalization-mode")).toBeVisible();
}

/** From Step 2 (personalization mode already chosen if the caller wanted to): advances through
 *  Step 3 (workspace) and fills the Step 4 (review) session name, leaving Create enabled but not
 *  yet clicked so each test still decides when to submit. */
async function fillWorkspaceAndTitle(page: Page, title: string) {
  const nextButton = page.getByRole("button", { name: "下一步" });
  await nextButton.click(); // Step 2 -> Step 3.

  const projectPath = page.getByPlaceholder(/code.*project/);
  await projectPath.fill("D:\\example-workspace");
  await projectPath.press("Tab");
  // Next only enables once the async project-path validation this same fill triggers settles.
  await expect(nextButton).toBeEnabled({ timeout: 10_000 });
  await nextButton.click(); // Step 3 -> Step 4.

  await page.getByPlaceholder("新会话").fill(title);
  await expect(page.getByRole("button", { name: "创建", exact: true })).toBeEnabled();
}

test.describe("session personalization mode", () => {
  test("offers the three modes beside the workspace", async ({ page }) => {
    await openCreateDialog(page);

    const select = page.getByTestId("session-personalization-mode");
    await expect(select.locator("option")).toHaveText(["标准", "仅本项目", "临时"]);
    // The help line changes with the choice, so the user reads what the mode does before creating.
    await expect(page.getByTestId("session-personalization-mode-help")).toContainText("全部记忆");
    await select.selectOption("temporary");
    await expect(page.getByTestId("session-personalization-mode-help")).toContainText("不会被记住");
  });

  test("refuses project-only until there is a workspace, and says why", async ({ page }) => {
    await openCreateDialog(page);

    await expect(page.getByTestId("session-personalization-mode-blocked")).toBeVisible();
    await expect(page.getByTestId("session-personalization-mode").locator("option[value='project-only']")).toBeDisabled();

    // The workspace field is on Step 3 now; go there to fill it, then back to Step 2 (`hasWorkspace`
    // is draft state, not per-step state, so the selector reflects the change once re-shown).
    await page.getByRole("button", { name: "下一步" }).click();
    await page.getByPlaceholder(/code.*project/).fill("D:\\example-workspace");
    await page.getByPlaceholder(/code.*project/).press("Tab");
    await page.getByRole("button", { name: "上一步" }).click();

    await expect(page.getByTestId("session-personalization-mode-blocked")).toHaveCount(0);
    await expect(page.getByTestId("session-personalization-mode").locator("option[value='project-only']")).toBeEnabled();
  });

  test("badges a temporary session for as long as it is open", async ({ page }) => {
    await openCreateDialog(page);
    await page.getByTestId("session-personalization-mode").selectOption("temporary");
    await fillWorkspaceAndTitle(page, "临时会话");
    await page.getByRole("button", { name: "创建", exact: true }).click();

    const badge = page.getByTestId("session-personalization-badge-temporary");
    await expect(badge).toBeVisible();
    // Persistent rather than a toast: the fact is true for the whole session, and a user scrolling
    // back has no way to re-read a message that has gone.
    await expect(badge).toHaveAttribute("title", /\u4e0d\u4f7f\u7528\u4e5f\u4e0d\u8bb0\u5f55/);
  });

  test("badges nothing for a standard session", async ({ page }) => {
    await openCreateDialog(page);
    await fillWorkspaceAndTitle(page, "标准会话");
    await page.getByRole("button", { name: "创建", exact: true }).click();

    await expect(page.getByTestId("session-conversation-header")).toBeVisible();
    await expect(page.getByTestId("session-personalization-badge-temporary")).toHaveCount(0);
    await expect(page.getByTestId("session-personalization-badge-project-only")).toHaveCount(0);
  });

  test("keeps each session's mode when switching between them", async ({ page }) => {
    await openCreateDialog(page);
    await page.getByTestId("session-personalization-mode").selectOption("temporary");
    await fillWorkspaceAndTitle(page, "临时的那个");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByTestId("session-personalization-badge-temporary")).toBeVisible();

    await reopenCreateDialog(page);
    await fillWorkspaceAndTitle(page, "标准的那个");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByTestId("session-personalization-badge-temporary")).toHaveCount(0);

    await page.getByRole("button", { name: /临时的那个/ }).first().click();
    // Switching back has to bring the first session's own promise with it.
    await expect(page.getByTestId("session-personalization-badge-temporary")).toBeVisible();
  });

  test("leaves an existing session's mode alone when instructions change", async ({ page }) => {
    await openCreateDialog(page);
    await page.getByTestId("session-personalization-mode").selectOption("temporary");
    await fillWorkspaceAndTitle(page, "不该被改的会话");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByTestId("session-personalization-badge-temporary")).toBeVisible();

    // In-app navigation rather than `page.goto`: the mock keeps sessions in module memory, and a
    // full page load would destroy the session this test is about before it could be re-read.
    await page.getByRole("button", { name: /设置/ }).first().click();
    await page.getByRole("button", { name: "AI 个性化", exact: true }).click();
    await page.getByTestId("personalization-view-tab-instructions").click();
    await page.getByTestId("personalization-field-aboutUser").fill("在会话创建之后写的。");
    await page.getByTestId("personalization-save").click();
    await expect(page.getByTestId("personalization-dirty")).toBeHidden();

    await page.getByRole("button", { name: /返回/ }).first().click();
    // Policy is layered and changes over time; the session's mode was decided once and is not a
    // layer anything can edit.
    await expect(page.getByTestId("session-personalization-badge-temporary")).toBeVisible();
  });
});
