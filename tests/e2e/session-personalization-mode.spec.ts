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
 */
async function openCreateDialog(page: Page) {
  await page.goto("/");
  await reopenCreateDialog(page);
}

/** Opens the dialog again without navigating, which would take the mock's sessions with it. */
async function reopenCreateDialog(page: Page) {
  await page.getByRole("button", { name: /新建/ }).click();
  await expect(page.getByTestId("session-personalization-mode")).toBeVisible();
}

async function fillWorkspaceAndTitle(page: Page, title: string) {
  const projectPath = page.getByPlaceholder(/code.*project/);
  const sessionTitle = page.getByPlaceholder("新会话");
  await expect(async () => {
    await projectPath.fill("D:\\example-workspace");
    await projectPath.press("Tab");
    await sessionTitle.fill(title);
    await expect(page.getByRole("button", { name: "创建", exact: true })).toBeEnabled({
      timeout: 1_000,
    });
  }).toPass({ timeout: 10_000 });
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

    await page.getByPlaceholder(/code.*project/).fill("D:\\example-workspace");
    await page.getByPlaceholder(/code.*project/).press("Tab");

    await expect(page.getByTestId("session-personalization-mode-blocked")).toHaveCount(0);
    await expect(page.getByTestId("session-personalization-mode").locator("option[value='project-only']")).toBeEnabled();
  });

  test("badges a temporary session for as long as it is open", async ({ page }) => {
    await openCreateDialog(page);
    await fillWorkspaceAndTitle(page, "临时会话");
    await page.getByTestId("session-personalization-mode").selectOption("temporary");
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
    await fillWorkspaceAndTitle(page, "临时的那个");
    await page.getByTestId("session-personalization-mode").selectOption("temporary");
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
    await fillWorkspaceAndTitle(page, "不该被改的会话");
    await page.getByTestId("session-personalization-mode").selectOption("temporary");
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
