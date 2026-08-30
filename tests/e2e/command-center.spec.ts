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
});
