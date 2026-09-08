import { expect, test, type Page } from "@playwright/test";

async function openMissionControl(page: Page, theme: "futuristic" | "minimal" = "futuristic", width = 1440) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript((selectedTheme) => window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme: selectedTheme })), theme);
  await page.goto("/"); await page.getByRole("button", { name: "Inbox" }).click();
  await expect(page.getByTestId("inbox-sections")).toBeVisible();
  // The full console (filters, counts, detail facets) lives one disclosure deep inside Inbox.
  await page.getByRole("button", { name: "Mission Control console" }).click();
  await expect(page.getByTestId("mission-control")).toBeVisible();
}

test("monitors multiple Runs, attention, failure, bounded filters, detail, and review navigation", async ({ page }) => {
  await openMissionControl(page);
  // Inbox lists the same runs above the console, so every lookup is scoped to the console.
  const console = page.getByTestId("mission-control");
  await expect(console.getByText("Attention inbox")).toBeVisible();
  await expect(console.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290").first()).toContainText("Waiting approval");
  await expect(console.getByText("provider_backoff", { exact: true })).toBeVisible();
  await expect(console.locator("[data-runner='ssh']").first()).toContainText("build.example.test");
  await console.getByLabel("Filter by Runner").selectOption("ssh");
  await console.getByLabel("Filter by status").selectOption("failed");
  const failed = console.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294").first();
  await expect(failed).toContainText("Runner interrupted"); await failed.locator("button").first().click();
  await expect(console.getByRole("tab", { name: "Overview" })).toBeVisible();
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
    const console = page.getByTestId("mission-control");
    await console.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a291").first().locator("button").first().click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    await expect(console.getByLabel("Filter by status")).toBeVisible();
    await expect(console.getByLabel("Filter by Runner")).toBeVisible();
    await expect(console.locator("[data-runner='ssh']").first()).toContainText("SSH");
    await expect(console.getByText("user_question", { exact: true }).first()).toBeVisible();
    await expect(console.getByRole("tablist")).toBeAttached();
    await expect(console.getByText("Select a Run to inspect available details.")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.getByTestId("mission-control").evaluate((element) => {
      const bounds = element.getBoundingClientRect();
      return bounds.height > 0 && bounds.width > 0;
    })).toBe(true);
    await page.getByTestId("mission-control").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
