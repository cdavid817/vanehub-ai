import { expect, test } from "@playwright/test";

test.describe("global command center", () => {
  test("opens with Ctrl/Cmd+K, navigates via a command, and Escape closes without navigating", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/workspace\/sessions/);
    // `MainLayout` is a lazy-loaded chunk: the URL updates as soon as the router matches, but the
    // component — and the effect that attaches the Ctrl/Cmd+K listener — can still be mounting.
    // Waiting for its own root proves the listener is live before the shortcut is pressed.
    await expect(page.getByTestId("workspace-frame")).toBeVisible();

    await page.keyboard.press("ControlOrMeta+k");
    const dialog = page.getByRole("dialog", { name: "命令中心" });
    await expect(dialog).toBeVisible();

    // Scoped to the dialog: Sessions has its own `combobox`/`option`-role elements elsewhere on the
    // page (e.g. the agent picker), so an unscoped `page.getByRole` is ambiguous once the palette
    // is open on top of it.
    await dialog.getByRole("combobox").fill("运行");
    await dialog.getByRole("option", { name: "前往运行" }).click();
    await expect(page).toHaveURL(/\/workspace\/runs\/attention$/);
    await expect(dialog).not.toBeVisible();

    // A second open, this time closed with Escape rather than a selection — proves Escape does not
    // also navigate anywhere, just dismisses.
    await page.keyboard.press("ControlOrMeta+k");
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
    await expect(page).toHaveURL(/\/workspace\/runs\/attention$/);
  });

  test("returns focus to the element that had it before the shortcut opened the dialog", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/workspace\/sessions/);
    await expect(page.getByTestId("workspace-frame")).toBeVisible();

    const runsButton = page.getByRole("button", { name: "运行", exact: true });
    await runsButton.focus();
    await page.keyboard.press("ControlOrMeta+k");
    await expect(page.getByRole("dialog", { name: "命令中心" })).toBeVisible();
    await page.keyboard.press("Escape");

    await expect(runsButton).toBeFocused();
  });

  test("searches for a goal by title and navigates to it, closing the dialog", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/workspace\/sessions/);
    await expect(page.getByTestId("workspace-frame")).toBeVisible();

    // Goal Center has no pre-seeded mock data (goal-center.spec.ts's own established pattern) --
    // create a real goal via the UI first so there is something for the provider to find.
    await page.getByRole("button", { name: "计划", exact: true }).click();
    await page.getByRole("tab", { name: "目标中心" }).click();
    await page.getByRole("button", { name: "新建目标" }).click();
    await page.getByLabel("标题").fill("命令中心可搜索目标");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByRole("heading", { level: 2, name: "命令中心可搜索目标" })).toBeVisible();

    // Navigate away first so the later URL assertion proves the Command Center itself moved us
    // there, not that we simply never left. Sessions has no activity-bar button of its own (it is
    // the persistent default view, not a destination entry like the other four) -- Quality is a
    // real button and conceptually unrelated to Plan/Goals.
    await page.getByRole("button", { name: "质量", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/quality/);

    await page.keyboard.press("ControlOrMeta+k");
    const dialog = page.getByRole("dialog", { name: "命令中心" });
    await expect(dialog).toBeVisible();
    await dialog.getByRole("combobox").fill("命令中心可搜索目标");
    await dialog.getByRole("option", { name: "命令中心可搜索目标", exact: true }).click();

    await expect(page).toHaveURL(/\/workspace\/plan\/goals\/.+/);
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole("heading", { level: 2, name: "命令中心可搜索目标" })).toBeVisible();
  });
});
