import { expect, test, type Page } from "@playwright/test";

/**
 * Mission Control is a Runs section now (runs-destination.tsx), not its own activity-bar entry —
 * clicking "Runs" already lands on its default "attention" section, where Mission Control renders.
 */
async function openMissionControl(page: Page, theme: "futuristic" | "minimal" = "futuristic", width = 1440) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript((selectedTheme) => window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme: selectedTheme })), theme);
  await page.goto("/"); await page.getByRole("button", { name: "Runs", exact: true }).click();
  await expect(page.getByTestId("mission-control")).toBeVisible();
}

test("monitors multiple Runs, attention, failure, bounded filters, detail, and review navigation", async ({ page }) => {
  await openMissionControl(page);
  // "Attention inbox" now also names the Runs tab that got here (runs-destination.tsx) — this
  // checks the section heading inside Mission Control's own content actually rendered.
  await expect(page.getByRole("heading", { name: "Attention inbox" })).toBeVisible();
  await expect(page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290").first()).toContainText("Waiting approval");
  await expect(page.getByText("provider_backoff", { exact: true })).toBeVisible();
  await expect(page.locator("[data-runner='ssh']").first()).toContainText("build.example.test");
  await page.getByLabel("Filter by Runner").selectOption("ssh");
  await page.getByLabel("Filter by status").selectOption("failed");
  const failed = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294").first();
  await expect(failed).toContainText("Runner interrupted"); await failed.locator("button").first().click();
  await expect(page.getByRole("tab", { name: "Overview" })).toBeVisible();
  await failed.locator("[data-action='review']").click();
  await expect(page).toHaveURL(/\/workspace\/sessions\//);
});

test("4.8: returns to the same run, with the same filter, after an evidence-navigation round trip", async ({ page }) => {
  await openMissionControl(page);
  await page.getByLabel("Filter by status").selectOption("failed");
  const failed = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294").first();
  await failed.locator("[data-action='review']").click();
  await expect(page).toHaveURL(/\/workspace\/sessions\//);

  const returnButton = page.getByRole("button", { name: "Back to Mission Control" });
  await expect(returnButton).toBeVisible();
  await returnButton.click();

  await expect(page).toHaveURL(/\/workspace\/runs\/attention\/018f0f17-4d6a-7e20-b41d-66c5271a294$/);
  await expect(page.getByTestId("mission-control")).toBeVisible();
  // Selected entity: the same run is restored into the detail pane, not just the URL.
  await expect(page.locator("aside").getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294")).toBeVisible();
  // Filter: the status filter set before leaving survives the round trip too.
  await expect(page.getByLabel("Filter by status")).toHaveValue("failed");
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
    await expect(page.getByLabel("Filter by status")).toBeVisible();
    await expect(page.getByLabel("Filter by Runner")).toBeVisible();
    await expect(page.locator("[data-runner='ssh']").first()).toContainText("SSH");
    await expect(page.getByText("user_question", { exact: true }).first()).toBeVisible();
    // Two tablists exist now that Mission Control lives inside Runs' own tab bar (runs-destination.tsx)
    // — scoped to the detail pane's facet tabs specifically, the one this assertion always meant.
    await expect(page.locator("aside").getByRole("tablist")).toBeAttached();
    await expect(page.getByText("Select a Run to inspect available details.")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.getByTestId("mission-control").evaluate((element) => {
      const bounds = element.getBoundingClientRect();
      return bounds.height > 0 && bounds.width > 0;
    })).toBe(true);
    await page.getByTestId("mission-control").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
