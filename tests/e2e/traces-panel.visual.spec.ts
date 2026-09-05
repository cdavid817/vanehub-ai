import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

/**
 * The Traces panel rendered, rather than reasoned about.
 *
 * Everything about duration semantics and event coverage was checked in jsdom, which has no layout
 * engine: it can read the classes that decide a box but never the box. These states are the ones
 * that only exist once something is laid out — a run list that must scroll rather than push the
 * panel past the viewport, a bar whose treatment has to read as "not a measurement" in both
 * themes, and a truncation notice sitting beside its own continuation control in a header that
 * also has to survive 390px.
 *
 * The states are not staged. The Web fixtures already carry an incomplete span with no derivable
 * duration and a run that reports a clipped event list, because those are exactly the states this
 * change taught the panel to distinguish.
 */

const VARIANTS = [
  { name: "futuristic-desktop", theme: "futuristic", width: 1440, height: 900 },
  { name: "minimal-desktop", theme: "minimal", width: 1440, height: 900 },
  { name: "futuristic-narrow", theme: "futuristic", width: 390, height: 844 },
  { name: "minimal-narrow", theme: "minimal", width: 390, height: 844 },
] as const;

test.describe.configure({ timeout: 120_000 });

async function useStyle(page: Page, theme: string) {
  await page.addInitScript((style) => {
    localStorage.setItem(
      "vanehub.appSettings",
      JSON.stringify({ applicationLanguage: "zh-CN", theme: style }),
    );
    localStorage.setItem("vanehub.uiStyle", style);
  }, theme);
}

async function openTraces(page: Page) {
  await page.goto("/");
  await createSession(page, "链路视觉验证");
  await page.getByRole("tab", { name: /链路|Traces/ }).click();
  await expect(page.getByRole("heading", { name: /执行时间线|Execution timeline/ })).toBeVisible();
}

/** Nothing may be reachable only by scrolling sideways. */
async function expectNoHorizontalSpill(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
}

test.describe("Traces panel visual", () => {
  for (const variant of VARIANTS) {
    test(`renders duration and coverage states in ${variant.name}`, async ({ page }) => {
      await useStyle(page, variant.theme);
      await page.setViewportSize({ width: variant.width, height: variant.height });
      await openTraces(page);

      // The run whose event list is clipped. Both the claim and the way out of it must be present:
      // telling a reader events are missing without a control to reach them is the failure this
      // panel was corrected for.
      const truncated = page.getByText(/事件列表已达上限|stopped at its limit/);
      await expect(truncated).toBeVisible();
      const loadMore = page.getByRole("button", { name: /加载更多事件|Load more events/ });
      await expect(loadMore).toBeVisible();

      // A span that ended without a derivable duration. It must not be announced as running, and
      // the accessible name is where that distinction actually reaches a reader.
      const spans = page.getByRole("listitem");
      const labels = await spans.evaluateAll((nodes) =>
        nodes.map((node) => node.getAttribute("aria-label") ?? ""),
      );
      expect(labels.some((label) => /耗时未知|Duration unknown/.test(label))).toBe(true);
      const incompleteLabels = labels.filter((label) => /不完整|Incomplete/.test(label));
      expect(incompleteLabels.length).toBeGreaterThan(0);
      for (const label of incompleteLabels) {
        expect(label).not.toMatch(/仍在运行|Still running/);
      }

      // The bars have to be *visible*, not merely correct. The time-axis column previously
      // resolved to 0px here — every bar was rendered at the right size into a column with no
      // width, so the waterfall showed no bars at all and nothing caught it.
      const geometry = await page.evaluate(() => {
        const rows = Array.from(document.querySelectorAll('[role="listitem"]'));
        return rows.map((row) => {
          const track = row.lastElementChild as HTMLElement | null;
          const bar = track?.querySelector("span[style]") as HTMLElement | null;
          return {
            trackWidth: track?.getBoundingClientRect().width ?? 0,
            barWidth: bar?.getBoundingClientRect().width ?? 0,
          };
        });
      });
      expect(geometry.length).toBeGreaterThan(0);
      for (const { trackWidth, barWidth } of geometry) {
        expect(trackWidth).toBeGreaterThan(0);
        // A bar wider than its own track reads as work continuing past the end of the run.
        expect(barWidth).toBeLessThanOrEqual(trackWidth + 1);
      }

      await expectNoHorizontalSpill(page);
      await page.screenshot({
        path: `test-results/traces-panel-${variant.name}.png`,
        fullPage: false,
      });
    });
  }

  test("following the continuation clears the truncation notice", async ({ page }) => {
    await useStyle(page, "futuristic");
    await page.setViewportSize({ width: 1440, height: 900 });
    await openTraces(page);

    const truncated = page.getByText(/事件列表已达上限|stopped at its limit/);
    await expect(truncated).toBeVisible();
    await page.getByRole("button", { name: /加载更多事件|Load more events/ }).click();

    // Reaching the end of the events must stop the panel claiming any are missing — a notice that
    // survived its own remedy would train readers to ignore it.
    await expect(truncated).toBeHidden();
    await expect(
      page.getByRole("button", { name: /加载更多事件|Load more events/ }),
    ).toBeHidden();
  });

  test("the run list scrolls instead of pushing the panel past the viewport", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openTraces(page);

    // The list sits in a flex column beside the newer-run notice. Without an explicit flex basis it
    // grows to its content, and a session with many runs pushes the whole panel off screen.
    const overflows = await page.evaluate(() => {
      // Located through its own heading: the page has several `aside` elements, and the first one
      // in the document is the session sidebar, not this list.
      const heading = Array.from(document.querySelectorAll("aside h2")).find((node) =>
        /执行记录|^Runs$/.test(node.textContent ?? ""),
      );
      const list = heading?.closest("aside") ?? null;
      if (!list) return null;
      const style = window.getComputedStyle(list);
      return {
        overflowY: style.overflowY,
        withinViewport: list.getBoundingClientRect().bottom <= window.innerHeight + 1,
      };
    });
    expect(overflows).not.toBeNull();
    expect(overflows?.overflowY).toBe("auto");
    expect(overflows?.withinViewport).toBe(true);
  });
});
