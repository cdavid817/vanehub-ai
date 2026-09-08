import { expect, test } from "@playwright/test";

test.describe("workspace help entry", () => {
  test("opens the bundled documentation rather than the About page", async ({ page }) => {
    await page.goto("/");
    // Help left the activity bar; it is pinned in the settings sidebar's bottom group.
    await expect(page.getByRole("navigation", { name: "工作区导航" }).getByRole("button", { name: "帮助" })).toHaveCount(0);
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await expect(page).toHaveURL(/\/settings$/);
    const bottomGroup = page.locator("[data-settings-group='bottom']");
    await bottomGroup.getByRole("button", { name: "使用文档", exact: true }).click();

    // The Help entry used to land on About, which made it look like a duplicate of Settings.
    await expect(page.getByRole("heading", { level: 2, name: "使用文档" })).toBeVisible();
    // The deep link that the old activity-bar entry produced still opens the same page.
    await page.goto("/settings?section=help");
    await expect(page.getByRole("heading", { level: 2, name: "使用文档" })).toBeVisible();
    // The README's own top-level heading proves the bundled document reached the page.
    await expect(page.getByRole("heading", { name: "VaneHub AI", exact: true })).toBeVisible();

    const navigation = page.locator("nav");
    const documentation = navigation.getByRole("button", { name: "使用文档", exact: true });
    await expect(documentation).toBeVisible();
    // The selected entry is highlighted; the highlight has to fit inside the sidebar rather than
    // being clipped by its scroll container.
    const entry = await documentation.boundingBox();
    const sidebar = await page.locator("aside").first().boundingBox();
    expect(entry).not.toBeNull();
    expect(sidebar).not.toBeNull();
    expect(entry!.x).toBeGreaterThanOrEqual(sidebar!.x - 1);
    expect(entry!.x + entry!.width).toBeLessThanOrEqual(sidebar!.x + sidebar!.width + 1);
  });
});
