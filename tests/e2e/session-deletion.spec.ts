import { expect, test, type Page } from "@playwright/test";

/**
 * The confirmed deletion flow against the Web/mock adapter. Every result here is *simulated*:
 * the mock never touches a filesystem, and the dialog says so. What these tests prove is the
 * interaction contract — default keep, explicit choice, blocked rows, batch grouping — not that
 * a directory was removed.
 */

async function createWorktreeSession(page: Page, title: string, worktreeName: string) {
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
  await page.getByLabel("创建新 Git worktree").check();
  await page.getByPlaceholder("feature-a").fill(worktreeName);
  await createButton.click();
  await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled();
}

async function createProjectSession(page: Page, title: string) {
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

function sessionCard(page: Page, title: string) {
  return page.locator("[data-session-id]").filter({ hasText: title });
}

async function openDeleteDialog(page: Page, title: string) {
  await sessionCard(page, title).first().click({ button: "right" });
  await page.getByRole("button", { name: "删除", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: /删除会话/ });
  await expect(dialog).toBeVisible();
  return dialog;
}

test("a project session confirms with no worktree option and cancelling deletes nothing", async ({ page }) => {
  const title = `项目会话-${Date.now()}`;
  await page.goto("/");
  await createProjectSession(page, title);

  const dialog = await openDeleteDialog(page, title);
  await expect(dialog.getByTestId("session-deletion-project-note")).toContainText("项目目录及其中的文件不会被删除");
  await expect(dialog.getByTestId("session-deletion-remove-worktree")).toHaveCount(0);
  await expect(dialog.getByTestId("session-deletion-confirm")).toHaveText("仅删除会话");
  await expect(dialog.getByTestId("session-deletion-cancel")).toBeFocused();

  await dialog.getByTestId("session-deletion-cancel").click();
  await expect(dialog).toHaveCount(0);
  await expect(sessionCard(page, title)).toHaveCount(1);
});

test("a worktree session keeps its directory by default and reports a simulated cleanup when chosen", async ({ page }) => {
  const title = `工作树会话-${Date.now()}`;
  await page.goto("/");
  await createWorktreeSession(page, title, "feature-clean");

  const dialog = await openDeleteDialog(page, title);
  await expect(dialog.getByTestId("session-deletion-simulated")).toBeVisible();
  await expect(dialog.getByTestId("session-deletion-worktree-path")).toContainText("example-workspace-feature-clean");
  await expect(dialog).toContainText("vanehub/feature-clean");
  const remove = dialog.getByTestId("session-deletion-remove-worktree");
  await expect(remove).not.toBeChecked();
  await expect(dialog.getByTestId("session-deletion-confirm")).toHaveText("仅删除会话");

  await remove.check();
  await expect(dialog.getByTestId("session-deletion-confirm")).toHaveText("删除会话及 worktree");
  await dialog.getByTestId("session-deletion-confirm").click();

  const result = dialog.getByTestId("session-deletion-result");
  await expect(result).toHaveAttribute("data-outcome", "succeeded");
  await expect(result).toContainText("模拟");
  await expect(result).toContainText("已移除");
  await dialog.getByTestId("session-deletion-cancel").click();
  await expect(sessionCard(page, title)).toHaveCount(0);
});

test("a dirty worktree cannot be cleaned up and the reason stays visible", async ({ page }) => {
  const title = `脏工作树-${Date.now()}`;
  await page.goto("/");
  await createWorktreeSession(page, title, "dirty-fix");

  const dialog = await openDeleteDialog(page, title);
  await expect(dialog.getByTestId("session-deletion-remove-worktree")).toBeDisabled();
  await expect(dialog.getByTestId("session-deletion-worktree-blockers")).toContainText("有未提交的已跟踪修改");
  await expect(dialog.getByTestId("session-deletion-worktree-status")).toContainText("有未提交修改");
  await expect(dialog.getByTestId("session-deletion-confirm")).toHaveText("仅删除会话");
  await dialog.getByTestId("session-deletion-confirm").click();
  await expect(dialog.getByTestId("session-deletion-result")).toHaveAttribute("data-outcome", "succeeded");
  await expect(dialog.getByTestId("session-deletion-result")).toContainText("已保留");
});

test("an ignored inventory needs its own acknowledgement before cleanup is allowed", async ({ page }) => {
  const title = `忽略文件-${Date.now()}`;
  await page.goto("/");
  await createWorktreeSession(page, title, "ignored-config");

  const dialog = await openDeleteDialog(page, title);
  await dialog.getByTestId("session-deletion-remove-worktree").check();
  await expect(dialog.getByTestId("session-deletion-ignored")).toContainText(".env");
  await expect(dialog.getByTestId("session-deletion-confirm")).toBeDisabled();
  await dialog.getByTestId("session-deletion-acknowledge-ignored").check();
  await expect(dialog.getByTestId("session-deletion-confirm")).toBeEnabled();
  // Unticking removal drops the acknowledgement: consent is per attempt.
  await dialog.getByTestId("session-deletion-remove-worktree").uncheck();
  await expect(dialog.getByTestId("session-deletion-ignored")).toHaveCount(0);
  await dialog.getByTestId("session-deletion-remove-worktree").check();
  await expect(dialog.getByTestId("session-deletion-acknowledge-ignored")).not.toBeChecked();
});

test("batch deletion opens the same dialog, groups by worktree, and keeps failed targets selected", async ({ page }) => {
  const stamp = Date.now();
  const good = `批量-正常-${stamp}`;
  const refused = `批量-拒绝-${stamp}`;
  await page.goto("/");
  await createWorktreeSession(page, good, `batch-good-${stamp}`);
  await createWorktreeSession(page, refused, `refuse-${stamp}`);

  await page.getByRole("button", { name: "更多操作" }).click();
  await page.getByRole("menuitem", { name: "批量管理" }).click();
  await page.getByRole("button", { name: "全选当前" }).click();
  await page.getByRole("button", { name: "批量删除" }).click();

  const dialog = page.getByRole("dialog", { name: /删除 \d+ 个会话/ });
  await expect(dialog).toBeVisible();
  const rows = dialog.getByTestId("session-deletion-worktree");
  await expect(rows).toHaveCount(2);
  for (const row of await rows.all()) await row.getByTestId("session-deletion-remove-worktree").check();
  await dialog.getByTestId("session-deletion-confirm").click();

  const result = dialog.getByTestId("session-deletion-result");
  await expect(result).toHaveAttribute("data-outcome", "partial");
  await expect(result).toContainText("Git 拒绝移除该 worktree");
  await expect(dialog.getByTestId("session-deletion-retry")).toBeVisible();
  await dialog.getByTestId("session-deletion-cancel").click();
  await expect(sessionCard(page, good)).toHaveCount(0);
  const remaining = sessionCard(page, refused);
  await expect(remaining).toHaveCount(1);
  // The batch is still open and the failed target is still selected.
  await expect(remaining.getByRole("checkbox")).toBeChecked();
});
