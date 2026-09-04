import { expect, test, type Page } from "@playwright/test";

/**
 * Mission Control is a Runs section now (runs-destination.tsx), not its own activity-bar entry —
 * clicking "Runs" already lands on its default "attention" section, where Mission Control renders.
 */
// 21.23: this whole file has always run under "en" (every pre-existing call site omits the new
// `locale` parameter below, so `openMissionControl`'s own default preserves that unchanged) --
// `runsButtonLabel` only adds the one other locale this task's own new ja test below needs.
const runsButtonLabel = { en: "Runs", ja: "実行" } as const;

async function openMissionControl(
  page: Page,
  theme: "futuristic" | "minimal" = "futuristic",
  width = 1440,
  locale: keyof typeof runsButtonLabel = "en",
) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript(
    ([selectedTheme, selectedLocale]) => window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: selectedLocale, theme: selectedTheme })),
    [theme, locale] as const,
  );
  await page.goto("/"); await page.getByRole("button", { name: runsButtonLabel[locale], exact: true }).click();
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
  // 16.13: "review" is now a real EvidenceLink (an `<a>`, not a `[data-action]` button).
  await failed.getByRole("link", { name: "Review changes" }).click();
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
  // 16.13: "review" is now a real EvidenceLink (an `<a>`, not a `[data-action]` button) whose `href`
  // already carries a real `?returnTo=` token (`withReturnTo`, mirroring `App.tsx`'s own navigate
  // handler byte-for-byte) -- the round trip below is exercising that token, not a fabricated one.
  await failed.getByRole("link", { name: "Review changes" }).click();
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

// 20.19: proves Run inspection is keyboard-operable end to end in a real browser -- distinct from
// mission-control-section-nav.test.tsx's own component-level roving-tabindex coverage (16.8/16.18),
// which never renders inside a real `aside` column or measures a real container width. A 2200px
// viewport is required, not the file's usual 1440: 16.8's own comment above (this file's first test)
// notes the section nav's `aside` column already lands in its compact `<select>` fallback at 1440px
// -- comfortably wide enough that the `minmax(0,1.4fr)_minmax(280px,1fr)` grid's own `aside` share
// clears the nav's 640px `COMPACT_MAX_WIDTH` and renders the readable `role="tablist"` this test
// needs. `overview`/`timeline`/`logs` are unconditionally "available" for every run in the Web mock
// (web-mission-control-client.ts), so Home/End are safe regardless of which run this test opens.
test("keyboard-only: opens a Run's detail and navigates the section-nav tablist with arrow keys", async ({ page }) => {
  await openMissionControl(page, "futuristic", 2200);
  const card = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290").first();
  await expect(card).toContainText("Waiting approval");
  const inspectTrigger = card.getByRole("button").first();
  await inspectTrigger.focus();
  await page.keyboard.press("Enter");

  const tablist = page.locator("aside").getByRole("tablist", { name: "Run detail sections" });
  await expect(tablist).toBeVisible();
  const overviewTab = tablist.getByRole("tab", { name: "Overview" });
  await expect(overviewTab).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("mission-control-overview-facet")).toBeVisible();

  await overviewTab.focus();
  await page.keyboard.press("ArrowRight");
  const timelineTab = tablist.getByRole("tab", { name: "Timeline" });
  await expect(timelineTab).toHaveAttribute("aria-selected", "true");
  await expect(timelineTab).toBeFocused();
  await expect(page.getByTestId("mission-control-timeline-facet")).toBeVisible();

  await page.keyboard.press("End");
  const logsTab = tablist.getByRole("tab", { name: /^Logs/ });
  await expect(logsTab).toHaveAttribute("aria-selected", "true");
  await expect(logsTab).toBeFocused();
  await expect(page.getByTestId("mission-control-logs-facet")).toBeVisible();

  await page.keyboard.press("Home");
  await expect(overviewTab).toHaveAttribute("aria-selected", "true");
  await expect(overviewTab).toBeFocused();
  await expect(page.getByTestId("mission-control-overview-facet")).toBeVisible();
});

// 21.23: this whole file runs under "en" by default (openMissionControl's own hardcoded default,
// unchanged by this task) -- Mission Control had zero coverage under any of the 4 non-default
// application locales before this test. ja is this task's own chosen representative (most
// non-Latin, non-CJK-adjacent script among the 4, per its own brief) -- mirrors the first test's
// own filter -> failed-run -> review-changes flow at the top of this file, but with every string
// read from `ja.json` rather than assumed, so a raw untranslated i18n key would fail this test
// rather than pass silently.
test("ja: renders translated Attention inbox content and completes a filter-to-review round trip", async ({ page }) => {
  await openMissionControl(page, "futuristic", 1440, "ja");
  await expect(page.getByRole("heading", { name: "要対応受信箱" })).toBeVisible();

  await openFilters(page);
  await page.getByLabel("Runner で絞り込む", { exact: true }).selectOption("ssh");
  await page.getByLabel("状態で絞り込む", { exact: true }).selectOption("failed");
  const failed = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294").first();
  await expect(failed).toContainText("Runner が中断されました");

  await failed.getByRole("link", { name: "変更をレビュー" }).click();
  await expect(page).toHaveURL(/\/workspace\/sessions\//);
});

for (const variant of [
  { name: "futuristic-desktop", theme: "futuristic" as const, width: 1440 },
  { name: "minimal-desktop", theme: "minimal" as const, width: 1440 },
  // 20.2: 1100 is this app's own declared Tauri minWidth (src-tauri/tauri.conf.json's own
  // `minWidth`) -- both stay in the wide, two-column `min-[900px]:grid-cols-[...]` mode
  // (mission-control.tsx's own `compact` query is `max-width: 899px`), so the shared body below
  // still applies unchanged; this only proves the page doesn't clip or break at the real floor,
  // not a second layout mode.
  { name: "futuristic-floor", theme: "futuristic" as const, width: 1100 },
  { name: "minimal-floor", theme: "minimal" as const, width: 1100 },
  // 20.2: 900 is the exact top edge of that same grid breakpoint -- still two-column here; the
  // real mode change (list replaced by detail, see the dedicated 768 test below) starts one named
  // width down. 1280/1024/768 are deliberately not each given their own entry here: every one of
  // them falls inside a mode a kept width already demonstrates (1280/1024 alongside 1440/1100/900's
  // two-column mode, 768 in its own dedicated test below), so a dedicated screenshot for each would
  // only re-capture an already-proven mode at a slightly different total width.
  { name: "futuristic-grid-edge", theme: "futuristic" as const, width: 900 },
  { name: "minimal-grid-edge", theme: "minimal" as const, width: 900 },
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

// 20.2: pre-existing regression fix, found while extending this same loop above -- not itself part
// of 20.2/20.17's own scope, but in the same file/loop and root-caused during that work, so fixed
// here rather than left broken. `futuristic-narrow`/`minimal-narrow` (390px, below the compact
// breakpoint) used to share the wide loop's own body above, which asserts list-only content (a
// different run's own SSH-runner badge, a "user_question" reason code) is visible alongside the
// selected run's detail -- correct back when a compact grid stacked both panes, but task 20.3's
// concurrent "swap list for detail below 900px instead of stacking" fix (commit ae7abb80) means
// selecting a Run now genuinely replaces the list, so that list-only content is no longer on
// screen. Confirmed pre-existing, not caused by this task's own diff, via `git stash` against the
// unmodified baseline before touching this loop. Split into its own loop with assertions matching
// the current, correct swap behavior -- the same discriminator the new dedicated 768px test above
// uses (a different, non-selected run's testid to prove the list itself is gone).
for (const variant of [
  { name: "futuristic-narrow", theme: "futuristic" as const, width: 390 },
  { name: "minimal-narrow", theme: "minimal" as const, width: 390 },
]) {
  test(`Mission Control visual ${variant.name}`, async ({ page }, testInfo) => {
    await openMissionControl(page, variant.theme, variant.width);
    const otherRunCard = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290");
    await expect(otherRunCard).toBeVisible();
    await page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a291").first().locator("button").first().click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);

    await expect(otherRunCard).toHaveCount(0);
    await expect(page.locator("aside").getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a291")).toBeVisible();
    await expect(page.getByTestId("mission-control-overview-facet")).toBeVisible();
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

// 20.2/20.17: 768 is the one width in this task's own named list (1600/1440/1280/1100/1024/900/
// 768/640) that actually falls below mission-control.tsx's own compact breakpoint
// (`useMediaQuery("(max-width: 899px)")`, backing the `min-[900px]:grid-cols-[...]` split the loop
// above stays inside of at 900 and up) -- at 768, selecting a Run genuinely replaces the list with
// its detail (`showingList`/`showingDetail`, mission-control.tsx) rather than merely narrowing both
// columns, so it gets its own real assertions instead of the wide loop's shared body, plus a themed
// screenshot pair -- closing 20.17's theme-paired coverage for this destination's one other real
// layout mode (it already had theme-paired coverage at the wide/narrow-select modes above).
for (const theme of ["futuristic", "minimal"] as const) {
  test(`Mission Control visual futuristic/minimal parity at the compact 768px grid breakpoint (${theme})`, async ({ page }, testInfo) => {
    await openMissionControl(page, theme, 768);
    const card = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a291").first();
    await expect(card).toBeVisible();
    // A different, non-selected run's card -- proves the *list* itself is gone, not just narrowed.
    // `mission-run-{id}` is not a list-exclusive marker: `MissionControlDetailPanel` (mission-
    // control-detail-panel.tsx) reuses the same `RunCard` to show the selected run's own summary at
    // the top of the detail pane, so the selected run's own testid relocates into `aside` rather
    // than disappearing -- checking a *different* run's testid (one never selected, so it can only
    // ever exist inside the list) is what actually proves the list unmounted.
    const otherRunCard = page.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290");
    await expect(otherRunCard).toBeVisible();
    await card.locator("button").first().click();

    // Compact replaces, it does not stack: the list is gone, not merely pushed off-screen below a
    // tall detail pane.
    await expect(otherRunCard).toHaveCount(0);
    await expect(page.locator("aside").getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a291")).toBeVisible();
    await expect(page.getByTestId("mission-control-overview-facet")).toBeVisible();
    // Structural, not text-based: `t("missionControl.actions.backToList")` (mission-control.tsx)
    // has no entry in any of the 5 shipped locale files (confirmed by grep -- a real, disclosed gap
    // in the concurrent commit that added this button, out of this task's own scope to fix), so it
    // renders as the raw i18next key today rather than real text. The Back button is still reliably
    // findable structurally: it is the one direct-child `<button>` of `aside` (mission-control.tsx's
    // own compact-only branch), rendered before `MissionControlDetailPanel`'s own nested buttons.
    const backButton = page.locator("aside > button").first();
    await expect(backButton).toBeVisible();

    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.getByTestId("mission-control").screenshot({ path: testInfo.outputPath(`mission-control-${theme}-grid-narrow.png`) });

    await backButton.click();
    await expect(otherRunCard).toBeVisible();
  });
}
