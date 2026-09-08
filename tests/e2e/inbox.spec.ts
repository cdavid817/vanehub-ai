import { expect, test, type Page } from "@playwright/test";

type Theme = "futuristic" | "minimal";

async function openInbox(page: Page, theme: Theme, width: number) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme: selectedTheme }));
    window.localStorage.setItem("vanehub.webSystemActivitySeed", JSON.stringify([
      { scopeKind: "workspace", canonicalScopeId: "e2e-workspace", eventCode: "breaker_opened", severity: "critical" },
      { scopeKind: "global", canonicalScopeId: "global", eventCode: "skill_created" },
    ]));
  }, theme);
  await page.goto("/");
  await page.getByRole("button", { name: "Inbox" }).click();
  await expect(page).toHaveURL(/\/workspace\/inbox\/attention$/);
  await expect(page.getByTestId("inbox-sections")).toBeVisible();
}

async function expectNoOverflowOrClipping(page: Page, rootTestId: string) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  const clipped = await page.getByTestId(rootTestId).evaluate((root) => {
    const viewportWidth = document.documentElement.clientWidth;
    // A control inside a horizontally scrolling strip is reachable by scrolling, not clipped.
    const scrollsHorizontally = (element: HTMLElement) => {
      for (let node = element.parentElement; node; node = node.parentElement) {
        const overflow = window.getComputedStyle(node).overflowX;
        if (overflow === "auto" || overflow === "scroll") return true;
      }
      return false;
    };
    return [...root.querySelectorAll<HTMLElement>("button, input, select, textarea")]
      .filter((element) => {
        // Off-canvas drawers are `visibility: hidden` while closed; they are not on screen to clip.
        const style = window.getComputedStyle(element);
        return element.offsetParent !== null && style.visibility !== "hidden" && !scrollsHorizontally(element);
      })
      .filter((element) => {
        const box = element.getBoundingClientRect();
        return box.left < -1 || box.right > viewportWidth + 1;
      })
      .map((element) => element.getAttribute("aria-label") ?? element.textContent?.trim() ?? element.tagName);
  });
  expect(clipped).toEqual([]);
}

test("composes attention runs and unread activity, and switches between list and board", async ({ page }) => {
  await openInbox(page, "futuristic", 1440);
  const attention = page.locator("#inbox-attention");
  await expect(attention.getByRole("heading", { name: "Needs attention" })).toBeVisible();
  await expect(attention.getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a290")).toBeVisible();
  await expect(attention.getByTestId("system-activity-item")).toHaveCount(1);
  await expect(attention.getByText("Breaker opened")).toBeVisible();
  await expect(page.locator("#inbox-running").getByRole("heading", { name: "Running" })).toBeVisible();
  await expect(page.locator("#inbox-recent").getByRole("heading", { name: "Recently finished" })).toBeVisible();
  // Two attention items plus the unread info event; the critical event is counted once.
  await expect(page.getByTestId("activity-bar-badge-inbox")).toBeVisible();

  await page.getByTestId("inbox-view-board").click();
  await expect(page).toHaveURL(/\/workspace\/inbox\/board$/);
  await expect(page.locator("#todo-board")).toBeVisible();
  await expect(page.getByTestId("inbox-sections")).toHaveCount(0);
  await page.reload();
  // The Board choice persists with the other layout preferences.
  await expect(page.locator("#todo-board")).toBeVisible();
  await page.getByTestId("inbox-view-list").click();
  await expect(page).toHaveURL(/\/workspace\/inbox\/attention$/);
  await expect(page.getByTestId("inbox-sections")).toBeVisible();
});

test("keeps row actions on their owning surfaces", async ({ page }) => {
  await openInbox(page, "futuristic", 1440);
  const failed = page.locator("#inbox-attention").getByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294");
  await failed.locator("[data-action='review']").click();
  await expect(page).toHaveURL(/\/workspace\/sessions\//);
  await expect(page.getByRole("tab", { name: "Changes" })).toHaveAttribute("aria-selected", "true");
});

for (const variant of [
  { name: "futuristic-desktop", theme: "futuristic" as const, width: 1440 },
  { name: "minimal-desktop", theme: "minimal" as const, width: 1440 },
  { name: "futuristic-narrow", theme: "futuristic" as const, width: 390 },
  { name: "minimal-narrow", theme: "minimal" as const, width: 390 },
]) {
  test(`Inbox visual ${variant.name}`, async ({ page }, testInfo) => {
    await openInbox(page, variant.theme, variant.width);
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    await expect(page.getByTestId("inbox-view-list")).toHaveAttribute("aria-pressed", "true");
    await expectNoOverflowOrClipping(page, "inbox");
    await page.getByTestId("inbox").screenshot({ path: testInfo.outputPath(`${variant.name}-list.png`) });

    await page.getByRole("button", { name: "Activity log" }).click();
    await expect(page.getByTestId("system-activity-view")).toBeVisible();
    await expectNoOverflowOrClipping(page, "inbox");
    await page.getByTestId("inbox").screenshot({ path: testInfo.outputPath(`${variant.name}-log.png`) });

    await page.getByTestId("inbox-view-board").click();
    await expect(page.locator("#todo-board")).toBeVisible();
    await expectNoOverflowOrClipping(page, "inbox");
    await page.getByTestId("inbox").screenshot({ path: testInfo.outputPath(`${variant.name}-board.png`) });
  });
}
