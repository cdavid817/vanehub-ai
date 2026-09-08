import { expect, test, type Page } from "@playwright/test";

type Theme = "futuristic" | "minimal";

async function openAutomations(page: Page, theme: Theme, width: number) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme: selectedTheme }));
  }, theme);
  await page.goto("/");
  await page.getByRole("button", { name: "Automations" }).click();
  await expect(page).toHaveURL(/\/workspace\/automations\/loops$/);
  await expect(page.getByTestId("automations")).toBeVisible();
}

async function expectNoOverflowOrClipping(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  const clipped = await page.getByTestId("automations").evaluate((root) => {
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

test("remembers the most recently used tab for the activity entry", async ({ page }) => {
  await openAutomations(page, "futuristic", 1440);
  await page.getByRole("tab", { name: "Goals" }).click();
  await expect(page).toHaveURL(/\/workspace\/automations\/goals$/);
  await page.getByRole("button", { name: "Inbox" }).click();
  await page.getByRole("button", { name: "Automations" }).click();
  await expect(page).toHaveURL(/\/workspace\/automations\/goals$/);
  await expect(page.getByRole("tab", { name: "Goals" })).toHaveAttribute("aria-selected", "true");
});

for (const variant of [
  { name: "futuristic-desktop", theme: "futuristic" as const, width: 1440 },
  { name: "minimal-desktop", theme: "minimal" as const, width: 1440 },
  { name: "futuristic-narrow", theme: "futuristic" as const, width: 390 },
  { name: "minimal-narrow", theme: "minimal" as const, width: 390 },
]) {
  test(`Automations visual ${variant.name}`, async ({ page }, testInfo) => {
    await openAutomations(page, variant.theme, variant.width);
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    await expect(page.locator("#loop-center")).toBeVisible();
    // The panel is visible before its lazy chunk lands; the screenshot must show the Loop Center.
    await expect(page.locator("#loop-center").getByText("Loading feature...")).toHaveCount(0);
    await expect(page.locator("#loop-center").getByRole("main")).toBeVisible();
    await expectNoOverflowOrClipping(page);
    await page.getByTestId("automations").screenshot({ path: testInfo.outputPath(`${variant.name}-loops.png`) });

    await page.getByRole("tab", { name: "Scheduled" }).click();
    await expect(page.getByTestId("scheduled-tasks-panel")).toBeVisible();
    await expect(page.getByLabel("Task name")).toBeFocused();
    await expectNoOverflowOrClipping(page);
    await page.getByTestId("automations").screenshot({ path: testInfo.outputPath(`${variant.name}-scheduled.png`) });

    await page.getByRole("tab", { name: "Goals" }).click();
    await expect(page.locator("#goal-center")).toBeVisible();
    await expectNoOverflowOrClipping(page);
    await page.getByTestId("automations").screenshot({ path: testInfo.outputPath(`${variant.name}-goals.png`) });
  });
}
