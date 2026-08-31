import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("session navigation", () => {
  test("filters the session list by search query", async ({ page }) => {
    const alpha = `搜索-alpha-${Date.now()}`;
    const beta = `搜索-beta-${Date.now()}`;
    await page.goto("/");
    await createSession(page, alpha);
    await createSession(page, beta);

    const search = page.locator("#workspace-session-search");
    await search.fill(alpha);

    await expect(page.getByRole("button", { name: new RegExp(alpha) })).toBeVisible();
    await expect(page.getByRole("button", { name: new RegExp(beta) })).toHaveCount(0);
  });

  test("returns focus to the toggle button after Escape-closing the narrow-layout session sheet", async ({ page }) => {
    await page.setViewportSize({ width: 640, height: 720 });
    await page.goto("/");
    const sessionSidebar = page.locator("#workspace-session-sidebar");

    // The sidebar starts open even at a narrow width (workbench-layout-preferences.ts's default),
    // so it already renders as a Sheet on load — collapsing it first gives a clean starting point
    // with a known trigger button, rather than a Sheet with no real "previous focus" to return to.
    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(sessionSidebar).toHaveCount(0);

    const expandToggle = page.getByRole("button", { name: "展开会话栏" });
    await expandToggle.click();
    await expect(sessionSidebar).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(sessionSidebar).toHaveCount(0);
    await expect(expandToggle).toBeFocused();
  });

  test("selects sessions in batch mode and deletes them", async ({ page }) => {
    const title = `批量-${Date.now()}`;
    await page.goto("/");
    await createSession(page, title);

    await page.getByRole("button", { name: "批量管理" }).click();
    const sessionCard = page.locator("[data-session-id]").filter({ hasText: title });
    await sessionCard.getByRole("checkbox", { name: "选择会话" }).check();

    const deleteButton = page.getByRole("button", { name: "批量删除" });
    await expect(deleteButton).toBeEnabled();
    await deleteButton.click();

    const confirmDialog = page.getByRole("dialog", { name: "批量删除会话" });
    await expect(confirmDialog).toBeVisible();
    await confirmDialog.getByRole("button", { name: "删除", exact: true }).click();

    await expect(confirmDialog).toHaveCount(0);
    await expect(page.locator("[data-session-id]").filter({ hasText: title })).toHaveCount(0);
  });
});
