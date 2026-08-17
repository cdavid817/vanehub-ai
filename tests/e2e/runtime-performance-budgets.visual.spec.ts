import { expect, test } from "@playwright/test";

const variants = [
  { name: "futuristic-desktop", theme: "futuristic", width: 1440, height: 900 },
  { name: "minimal-desktop", theme: "minimal", width: 1440, height: 900 },
  { name: "futuristic-narrow", theme: "futuristic", width: 390, height: 844 },
  { name: "minimal-narrow", theme: "minimal", width: 390, height: 844 },
] as const;

for (const variant of variants) {
  test(`bounded long-list visual ${variant.name}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: variant.width, height: variant.height });
    await page.addInitScript(({ theme }) => {
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "zh-CN", theme }));
      const hooks = Object.fromEntries(Array.from({ length: 501 }, (_, index) => {
        const id = `performance-visual-${String(index).padStart(3, "0")}`;
        return [id, {
          id,
          name: `Performance Hook ${index}`,
          description: `Bounded visual fixture ${index}`,
          category: "static",
          stage: "session-init",
          order: 2_000 + index,
          version: 1,
          source: "user",
          enabled: true,
          disableable: true,
          cliBindings: ["codex-cli"],
          governance: {
            safetyTier: "editable",
            transparencyTier: "opt-in-view",
            governanceTier: "human-gated",
          },
          templateBody: `Bounded prompt ${index}`,
          createdAt: "2026-08-17T00:00:00.000Z",
          updatedAt: "2026-08-17T00:00:00.000Z",
        }];
      }));
      window.localStorage.setItem("vanehub.prompt-hooks.v1", JSON.stringify(hooks));
    }, { theme: variant.theme });

    await page.goto("/settings");
    await page.getByRole("button", { name: "Prompt Hook" }).click();

    const list = page.getByTestId("prompt-hook-virtual-list");
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    await expect(list).toBeVisible();
    await expect.poll(async () => Number(await list.getAttribute("data-virtual-count"))).toBeGreaterThanOrEqual(250);
    await expect.poll(async () => Number(await list.getAttribute("data-rendered-count"))).toBeLessThan(30);
    await expect(list.getByRole("listitem").first()).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await list.evaluate((element) => element.clientHeight > 0 && element.clientWidth > 0)).toBe(true);
    await page.screenshot({ fullPage: true, path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
