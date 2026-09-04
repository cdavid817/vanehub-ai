import { expect, test } from "@playwright/test";

const variants = [
  { name: "futuristic-desktop", theme: "futuristic", width: 1440, height: 900 },
  { name: "minimal-desktop", theme: "minimal", width: 1440, height: 900 },
  { name: "futuristic-narrow", theme: "futuristic", width: 390, height: 844 },
  { name: "minimal-narrow", theme: "minimal", width: 390, height: 844 },
] as const;

for (const variant of variants) {
  test(`Skill tool governance ${variant.name}`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme }));
      localStorage.setItem("vanehub.uiStyle", theme);
    }, variant.theme);
    await page.setViewportSize({ width: variant.width, height: variant.height });
    await page.goto("/settings");
    // Below `lg` the sidebar is hidden in favor of a searchable sheet (task 12.9): open it first.
    if (variant.width < 1024) {
      await page.getByRole("button", { name: /^Switch settings page/ }).click();
    }
    await page.getByRole("button", { name: "Skills", exact: true }).click();
    const skill = page.locator('article[data-skill-id="code-review"]');
    await skill.getByRole("button", { name: /View details for/ }).click();
    const surface = variant.width >= 1280 ? page.getByRole("complementary", { name: /details/ }) : page.getByRole("dialog", { name: /details/ });
    await surface.getByRole("tab", { name: "Tools" }).click();
    const tool = surface.locator('article[data-tool-revision]');
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    await expect(tool).toContainText("Untrusted");
    await expect(tool).toContainText("Disabled");
    await expect(tool.getByRole("button", { name: "Trust revision" })).toBeDisabled();
    const overflow = await surface.evaluate((element) => element.scrollWidth > element.clientWidth + 1);
    expect(overflow).toBe(false);
    await surface.getByRole("tab", { name: "Tools" }).focus();
    await expect(surface.getByRole("tab", { name: "Tools" })).toBeFocused();
    await page.screenshot({ fullPage: true, path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
