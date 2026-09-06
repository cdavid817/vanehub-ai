import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

const list = '[data-testid="trace-waterfall-list"]';

test.describe("trace waterfall remount", () => {
  test("keeps rendering rows after a resize and after leaving and returning to the tab", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await createSession(page, "链路重挂载");
    await page.getByRole("tab", { name: /链路|Traces/ }).click();
    await expect(page.getByRole("heading", { name: /执行时间线|Execution timeline/ })).toBeVisible();
    await expect(page.locator(list)).not.toHaveAttribute("data-rendered-count", "0");

    // Shrinking to the desktop client's minimum window must not lose the rows.
    await page.setViewportSize({ width: 1100, height: 700 });
    await expect(page.locator(list)).not.toHaveAttribute("data-rendered-count", "0");

    // The list is remounted when the tab is left and reopened. It once sized its viewport from an
    // auto-height box, measured zero, rendered nothing, and so stayed at zero.
    await page.getByRole("tab", { name: /报告|Report/ }).click();
    await page.getByRole("tab", { name: /链路|Traces/ }).click();
    await expect(page.getByRole("heading", { name: /执行时间线|Execution timeline/ })).toBeVisible();
    await expect(page.locator(list)).not.toHaveAttribute("data-rendered-count", "0");
    await expect(page.getByText(/execute_tool search/)).toBeVisible();
  });
});
