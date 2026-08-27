import { expect, test } from "@playwright/test";

test.describe("workspace help entry", () => {
  test("opens the bundled documentation rather than the About page", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "帮助" }).click();

    // The Help entry used to land on About, which made it look like a duplicate of Settings.
    await expect(page).toHaveURL(/\/settings\?section=help$/);
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
