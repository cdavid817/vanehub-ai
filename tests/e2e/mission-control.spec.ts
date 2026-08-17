import { expect, test, type Page } from "@playwright/test";

async function openMissionControl(page: Page, theme: "futuristic" | "minimal" = "futuristic", width = 1440) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript((selectedTheme) => window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme: selectedTheme })), theme);
  await page.goto("/"); await page.getByRole("button", { name: "Mission Control" }).click();
  await expect(page.getByTestId("mission-control")).toBeVisible();
}

test("monitors multiple Runs, attention, failure, bounded filters, detail, and review navigation", async ({ page }) => {
  await openMissionControl(page);
  await expect(page.getByText("Attention inbox")).toBeVisible();
  await expect(page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290").first()).toContainText("Waiting approval");
  await expect(page.getByText("provider_backoff", { exact: true })).toBeVisible();
  await page.getByLabel("Filter by status").selectOption("failed");
  const failed = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294").first();
  await expect(failed).toContainText("verification_failed"); await failed.locator("button").first().click();
  await expect(page.getByRole("tab", { name: "Overview" })).toBeVisible();
  await failed.locator("[data-action='review']").click();
  await expect(page).toHaveURL(/\/workspace\/sessions\//);
});

for (const variant of [
  { name: "futuristic-desktop", theme: "futuristic" as const, width: 1440 },
  { name: "minimal-desktop", theme: "minimal" as const, width: 1440 },
  { name: "futuristic-narrow", theme: "futuristic" as const, width: 390 },
  { name: "minimal-narrow", theme: "minimal" as const, width: 390 },
]) {
  test(`Mission Control visual ${variant.name}`, async ({ page }, testInfo) => {
    await openMissionControl(page, variant.theme, variant.width);
    await page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a291").first().locator("button").first().click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.getByTestId("mission-control").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
