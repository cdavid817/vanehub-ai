import { expect, test } from "@playwright/test";
import { createAndRunLoop, openLoops } from "./loop-helpers";

const variants = [
  { drawer: null, height: 900, name: "futuristic-desktop", theme: "futuristic", width: 1440 },
  { drawer: null, height: 900, name: "minimal-desktop", theme: "minimal", width: 1440 },
  { drawer: "navigation", height: 844, name: "futuristic-narrow", theme: "futuristic", width: 390 },
  { drawer: "inspector", height: 844, name: "minimal-narrow", theme: "minimal", width: 390 },
] as const;

test.describe("Loop engineering visual verification", () => {
  for (const variant of variants) {
    test(`${variant.name} has bounded, nonblank Loop content`, async ({ page }) => {
      await page.setViewportSize({ width: variant.width, height: variant.height });
      await page.addInitScript((theme) => {
        window.localStorage.setItem(
          "vanehub.appSettings",
          JSON.stringify({ applicationLanguage: "zh-CN", theme }),
        );
        window.localStorage.setItem("vanehub.uiStyle", theme);
      }, variant.theme);
      await page.goto("/");
      await openLoops(page);
      await createAndRunLoop(page, `${variant.name} 循环`);

      const loopCenter = page.locator("#loop-center");
      const timeline = loopCenter.getByRole("main");
      await expect(timeline.getByText("等待验收", { exact: true }).first()).toBeVisible();
      await expect(timeline.getByText("验证检查")).toBeVisible();
      await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);

      if (variant.drawer === "navigation") {
        await page.getByRole("button", { name: "打开循环列表" }).click();
        await expect(page.locator("#loop-navigation-drawer")).toBeVisible();
      }
      if (variant.drawer === "inspector") {
        await page.getByRole("button", { name: "打开循环检查器" }).click();
        await expect(page.locator("#loop-inspector-drawer")).toBeVisible();
      }
      if (variant.drawer) await page.waitForTimeout(250);

      const geometry = await page.evaluate(() => {
        const root = document.querySelector<HTMLElement>("#loop-center");
        const viewportWidth = document.documentElement.clientWidth;
        const outOfBounds = root
          ? [...root.querySelectorAll<HTMLElement>("button, input, select, textarea")]
            .filter((element) => {
              const style = window.getComputedStyle(element);
              return element.offsetParent !== null && style.display !== "none" && style.visibility !== "hidden";
            })
            .filter((element) => {
              const box = element.getBoundingClientRect();
              return box.left < -1 || box.right > viewportWidth + 1 || box.width < 24 || box.height < 24;
            })
            .map((element) => element.getAttribute("aria-label") ?? element.textContent?.trim() ?? element.tagName)
          : ["missing-loop-center"];
        return {
          documentOverflow: document.documentElement.scrollWidth > viewportWidth,
          outOfBounds,
          textLength: root?.innerText.trim().length ?? 0,
        };
      });
      expect(geometry.documentOverflow).toBe(false);
      expect(geometry.outOfBounds).toEqual([]);
      expect(geometry.textLength).toBeGreaterThan(100);

      await page.screenshot({
        animations: "disabled",
        path: `test-results/loop-${variant.name}.png`,
      });
    });
  }
});
