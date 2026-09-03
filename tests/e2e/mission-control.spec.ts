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

// 16.5 moved Agent/status/Runner into a `FilterPopover` (mission-control-toolbar.tsx) whose fields
// only mount once its own "Filters" trigger is open (FilterPopover.tsx's `{open ? (...) : null}`) --
// every direct `getByLabel` lookup below needs this clicked first, and needs it called again after
// any click elsewhere on the page (FilterPopover closes itself on any outside pointerdown) or after
// any round trip through another destination (`MissionControl` fully remounts on every visit, per
// its own 4.8 comment, resetting the popover's local `open` state back to closed). Scoped to the
// `mission-control` testid and located by testid rather than role/name: the session sidebar mounts
// its own separate `FilterPopover` on this same page, and the trigger's own accessible name grows
// an "N active filter(s)" suffix once a field is set, which also becomes a substring match against
// the panel's separate "Clear filters" button.
async function openFilters(page: Page) {
  await page.getByTestId("mission-control").getByTestId("filter-popover-trigger").click();
}

test("monitors multiple Runs, attention, failure, bounded filters, detail, and review navigation", async ({ page }) => {
  await openMissionControl(page);
  // "Attention inbox" now also names the Runs tab that got here (runs-destination.tsx) — this
  // checks the section heading inside Mission Control's own content actually rendered.
  await expect(page.getByRole("heading", { name: "Attention inbox" })).toBeVisible();
  await expect(page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290").first()).toContainText("Waiting approval");
  await expect(page.locator("[data-runner='ssh']").first()).toContainText("build.example.test");
  await openFilters(page);
  await page.getByLabel("Filter by Runner", { exact: true }).selectOption("ssh");
  await page.getByLabel("Filter by status", { exact: true }).selectOption("failed");
  const failed = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294").first();
  await expect(failed).toContainText("Runner interrupted"); await failed.locator("button").first().click();
  // 16.8: the section nav's own container width, not the viewport, decides whether this renders as
  // a readable role="tab" strip or a compact <select> (mission-control-section-nav.tsx) -- checking
  // for the Overview facet's own real content, rendered either way, proves detail actually rendered
  // without depending on which nav variant this viewport happens to land on.
  await expect(page.getByTestId("mission-control-overview-facet")).toBeVisible();
  // 16.8: this viewport's own aside width lands the section nav in its compact <select> form (see
  // mission-control-section-nav.tsx) -- proves the fallback itself actually switches facets in a
  // real browser, not just in the component-level tests that force `compact={true}` directly.
  await page.getByLabel("Run detail sections").selectOption("timeline");
  await expect(page.getByTestId("mission-control-timeline-facet")).toBeVisible();
  await failed.locator("[data-action='review']").click();
  await expect(page).toHaveURL(/\/workspace\/sessions\//);
});

test("16.2: the Active/History route tabs scope Mission Control to their own bucket, not every Run at once", async ({ page }) => {
  await openMissionControl(page);
  await expect(page.getByRole("heading", { name: "Attention inbox" })).toBeVisible();
  // A "Retrying" run this fixture only places in the Active bucket, never Attention.
  await expect(page.getByText("provider_backoff", { exact: true })).toHaveCount(0);

  await page.getByRole("tab", { name: "Active Runs", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Active Runs" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Attention inbox" })).toHaveCount(0);
  await expect(page.getByText("provider_backoff", { exact: true })).toBeVisible();

  await page.getByRole("tab", { name: "Recently completed", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Recently completed" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Active Runs" })).toHaveCount(0);
});

test("4.8: returns to the same run, with the same filter, after an evidence-navigation round trip", async ({ page }) => {
  await openMissionControl(page);
  await openFilters(page);
  await page.getByLabel("Filter by status", { exact: true }).selectOption("failed");
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
  // Filter: the status filter set before leaving survives the round trip too. A fresh MissionControl
  // instance mounted on the way back in, so its own FilterPopover starts closed again.
  await openFilters(page);
  await expect(page.getByLabel("Filter by status", { exact: true })).toHaveValue("failed");
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
    // After the run-card click above, not before: FilterPopover closes itself on any outside click.
    await openFilters(page);
    await expect(page.getByLabel("Filter by status", { exact: true })).toBeVisible();
    await expect(page.getByLabel("Filter by Runner", { exact: true })).toBeVisible();
    await expect(page.locator("[data-runner='ssh']").first()).toContainText("SSH");
    await expect(page.getByText("user_question", { exact: true }).first()).toBeVisible();
    // Scoped to the detail pane's own section nav specifically -- Runs' own tab bar
    // (runs-destination.tsx) has an unrelated tablist outside this `aside`. 16.8: the detail pane's
    // section nav can render as either a readable role="tab" strip or a compact <select>, decided by
    // its own container width, so this checks the shared wrapper both variants render rather than
    // the tablist role, which only the readable variant has.
    await expect(page.locator("aside").getByTestId("mission-control-section-nav")).toBeAttached();
    await expect(page.getByText("Select a Run to inspect available details.")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.getByTestId("mission-control").evaluate((element) => {
      const bounds = element.getBoundingClientRect();
      return bounds.height > 0 && bounds.width > 0;
    })).toBe(true);
    await page.getByTestId("mission-control").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
